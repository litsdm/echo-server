use std::str::FromStr;

use actix_web::{
    HttpMessage, HttpRequest, post,
    web::{Data, Json},
};
use isolang::Language;
use serde::{Deserialize, Serialize};
use surrealitos::SurrealId;
use whatlang::{Lang, detect_lang};

use crate::{
    connector::{
        HttpMethod,
        backblaze::BackBlaze,
        mistral::Mistral,
        modal::{DiarizationInput, ModalAI, ToolAsyncIO},
    },
    error::Result,
    model::{
        Controller,
        token::Claims,
        transcription::{NewTranscription, Status, Transcription, TranscriptionController},
    },
    repo::surreal::SurrealDB,
};

#[derive(Deserialize, Serialize)]
pub struct FilePayload {
    file: String,
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

// TODO: organize this function and make it respond faster with more async functionality once we have websockets
// So basically create and respond with an empty transcription that will be populated once we transcribe and then diarize
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

    let mistral = Mistral::new();
    let modal = ModalAI::new();
    let raw_transcription = mistral.transcribe(&file_url, true).await?;

    let sanitized_url = if let Some(question_mark_pos) = file_url.find('?') {
        file_url[..question_mark_pos].to_string()
    } else {
        file_url.clone()
    };

    let language = match raw_transcription.language {
        Some(lang) => Some(lang),
        None => {
            let lang = detect_lang(&raw_transcription.text).unwrap_or(Lang::Eng);
            Language::from_639_3(lang.code())
                .unwrap()
                .to_639_1()
                .map(|s| s.to_string())
        }
    };

    let new_transcription = NewTranscription {
        status: Some(Status::Done),
        raw: Some(raw_transcription.text),
        audio_file: Some(sanitized_url),
        user: Some(user_id),
        language: language.clone(),
        ..Default::default()
    };

    let transcription = TranscriptionController::create(&db.surreal, &new_transcription).await?;

    let diarize_input = DiarizationInput {
        audio: file_url,
        segments: raw_transcription.segments.unwrap_or(vec![]),
        language: language.unwrap_or("en".to_string()),
        webhook_url: "".to_string(),
    };

    println!("{:?}", diarize_input);

    let output = modal
        .run::<DiarizationInput, ToolAsyncIO>(HttpMethod::Post, "diarize/initiate", &diarize_input)
        .await?;

    println!("{:?}", output);

    Ok(Json(transcription))
}
