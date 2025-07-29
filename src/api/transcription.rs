use std::str::FromStr;

use actix_web::{
    HttpMessage, HttpRequest, post,
    web::{Data, Json},
};
use serde::{Deserialize, Serialize};
use surrealitos::SurrealId;

use crate::{
    connector::{backblaze::BackBlaze, mistral::Mistral},
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
