use std::str::FromStr;

use actix_web::{
    HttpMessage, HttpRequest, get, post,
    web::{Data, Json, Path, Query},
};
use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::remote::ws::Client};
use surrealitos::SurrealId;

use crate::{
    api::{PaginationParameters, make_default_webhook_url},
    connector::{
        HttpMethod,
        backblaze::BackBlaze,
        mistral::{Mistral, TranscriptionResponse},
        modal::{
            BaseParameters, DiarizationInput, ModalAI, ResultOutput, Status as ModalStatus,
            ToolAsyncIO,
        },
        reverb::Reverb,
    },
    error::{Error, Result},
    model::{
        Controller,
        token::Claims,
        transcription::{
            NewTranscription, Segment, Status, Transcription, TranscriptionController,
            TranscriptionPatch,
        },
        user::UserController,
    },
    repo::surreal::SurrealDB,
};

#[derive(Deserialize, Serialize)]
pub struct FilePayload {
    file: String,
}

#[derive(Deserialize, Serialize)]
pub struct DiarizeOutput {
    pub segments: Vec<Segment>,
}

#[get("/all")]
pub async fn get_user_transcriptions(
    db: Data<SurrealDB>,
    query: Query<PaginationParameters>,
    req: HttpRequest,
) -> Result<Json<Vec<Transcription>>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    let user_id = SurrealId::from_str(&claims.sub)?;
    let user_option = UserController::get(&db.surreal, &user_id).await?;

    if user_option.is_none() {
        return Err(Error::WrongCredentials);
    }

    let transcriptions = user_option
        .unwrap()
        .get_transcriptions(&db.surreal, &query.into_inner())
        .await?;

    Ok(Json(transcriptions))
}

#[get("/{id}")]
pub async fn get_transcription(
    db: Data<SurrealDB>,
    path: Path<String>,
) -> Result<Json<Transcription>> {
    let id: SurrealId = path.into_inner().parse()?;
    let transcription_opt = TranscriptionController::get(&db.surreal, &id).await?;
    let transcription = transcription_opt.ok_or(Error::NotFound("transcription".to_string()))?;

    Ok(Json(transcription))
}

#[post("/raw")]
pub async fn transcribe_raw_only(
    db: Data<SurrealDB>,
    body: Json<FilePayload>,
    req: HttpRequest,
) -> Result<Json<Transcription>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    let user_id = SurrealId::from_str(&claims.sub)?;

    let payload = body.into_inner();
    let mut file_url = payload.file;

    if !file_url.contains("Authorization=") {
        let read_blaze_token = BackBlaze::get_read_auth_token().await?;
        let separator = if file_url.contains('?') { "&" } else { "?" };
        file_url = format!(
            "{}{}Authorization={}",
            file_url, separator, read_blaze_token
        );
    }

    let mistral = Mistral::new();
    let raw_transcription = mistral.transcribe(&file_url, false).await?;

    let sanitized_url = if let Some(question_mark_pos) = file_url.find('?') {
        file_url[..question_mark_pos].to_string()
    } else {
        file_url
    };

    let new_transcription = NewTranscription {
        status: Some(Status::Done),
        raw: Some(raw_transcription.text),
        audio_file: Some(sanitized_url),
        user: Some(user_id),
        ..Default::default()
    };

    let transcription = TranscriptionController::create(&db.surreal, &new_transcription).await?;

    Ok(Json(transcription))
}

async fn diarize_async(diarize_input: &DiarizationInput) -> Result<ToolAsyncIO> {
    // TODO: handle calls
    let modal = ModalAI::new();

    let output = modal
        .run::<DiarizationInput, ToolAsyncIO>(HttpMethod::Post, "diarize/initiate", diarize_input)
        .await?;

    Ok(output)
}

async fn transcribe_async(
    client: &Surreal<Client>,
    transcription_id: &SurrealId,
    file_url: &str,
    diarize: bool,
) -> Result<TranscriptionResponse> {
    let mistral = Mistral::new();
    let reverb = Reverb::new();
    let raw_transcription = mistral.transcribe(file_url, true).await?;
    println!(
        "raw_transcription: {}",
        serde_json::to_string_pretty(&raw_transcription)?
    );

    let new_status = if diarize {
        Status::Diarizing
    } else {
        Status::Done
    };

    let patch = TranscriptionPatch {
        raw: Some(raw_transcription.text.to_string()),
        language: raw_transcription.language.clone(),
        status: Some(new_status),
        ..Default::default()
    };

    let transcription =
        TranscriptionController::update(client, &transcription_id.to_string(), &patch).await?;

    let _ = reverb
        .notify_update("transcription/updated", transcription)
        .await;

    if diarize {
        let segments = raw_transcription.segments.clone();
        let diarize_input = DiarizationInput {
            audio: file_url.to_string(),
            segments: segments.unwrap_or(vec![]),
            base: BaseParameters {
                webhook_url: make_default_webhook_url("diarize"),
                job_id: transcription_id.to_string(),
            },
        };

        tokio::spawn(async move { diarize_async(&diarize_input).await });
    }

    Ok(raw_transcription)
}

#[post("/transcribe")]
pub async fn transcribe(
    db: Data<SurrealDB>,
    body: Json<FilePayload>,
    req: HttpRequest,
) -> Result<Json<Transcription>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    let user_id = SurrealId::from_str(&claims.sub)?;

    let payload = body.into_inner();
    let mut file_url = payload.file;

    if !file_url.contains("Authorization=") {
        let read_blaze_token = BackBlaze::get_read_auth_token().await?;
        let separator = if file_url.contains('?') { "&" } else { "?" };
        file_url = format!(
            "{}{}Authorization={}",
            file_url, separator, read_blaze_token
        );
    }

    let reverb = Reverb::new();

    let sanitized_url = if let Some(question_mark_pos) = file_url.find('?') {
        file_url[..question_mark_pos].to_string()
    } else {
        file_url.clone()
    };

    let new_transcription = NewTranscription {
        status: Some(Status::Transcribing),
        audio_file: Some(sanitized_url),
        user: Some(user_id),
        ..Default::default()
    };

    let transcription = TranscriptionController::create(&db.surreal, &new_transcription).await?;

    let id = transcription.id.clone();
    // fire and forget to continue in bg without blocking the API response for user
    tokio::spawn(async move { transcribe_async(&db.surreal, &id, &file_url, true).await });

    let _ = reverb
        .notify_update("transcription/created", transcription.clone())
        .await;

    Ok(Json(transcription))
}

#[post("/diarize/status")]
pub async fn diarize_webhook(
    db: Data<SurrealDB>,
    body: Json<ResultOutput<DiarizeOutput>>,
) -> Result<Json<String>> {
    let reverb = Reverb::new();
    let result = body.into_inner();
    if result.status == ModalStatus::Success {
        let id = result.id.clone().unwrap();
        if let Some(data) = result.data {
            // TODO: spawn summarization here

            let patch = TranscriptionPatch {
                status: Some(Status::Summarizing),
                diarized: Some(data.segments),
                ..Default::default()
            };

            let transcription =
                TranscriptionController::update(&db.surreal, &id.to_string(), &patch).await?;
            let _ = reverb
                .notify_update("transcription/updated", transcription)
                .await;
        }
    }

    Ok(Json("Success".to_string()))
}
