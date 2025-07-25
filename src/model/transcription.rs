use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use surrealdb::{Datetime, Surreal, engine::remote::ws::Client};
use surrealitos::{Relation, SurrealId, extract_id, serialize_as_optional_record};

use crate::{
    error::{Error, Result},
    model::{Controller, LLMProvider, user::User},
};

// TODO: add diarization params based on whisperx response
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Segment {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Status {
    Transcribing,
    Diarizing,
    Summarizing,
    Done,
    Fail,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Transcription {
    pub id: SurrealId,
    pub created_at: String,
    pub updated_at: String,
    pub status: Status,
    pub user: Relation<User>,
    pub raw: Option<String>,
    pub diarized: Option<Vec<Segment>>,
    pub note: Option<String>, // Summarized note
    pub llm: Option<String>,
    pub llm_provider: Option<LLMProvider>,
    pub audio_file: Option<String>, // B2 url
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct NewTranscription {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(alias = "audioFile", skip_serializing_if = "Option::is_none")]
    pub audio_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diarized: Option<Vec<Segment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<String>,
    #[serde(alias = "llmProvider", skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<LLMProvider>,

    #[serde(skip_deserializing, serialize_with = "serialize_as_optional_record")]
    pub user: Option<SurrealId>,

    #[serde(skip_deserializing)]
    pub created_at: Option<Datetime>,
    #[serde(skip_deserializing)]
    pub updated_at: Option<Datetime>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TranscriptionPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diarized: Option<Vec<Segment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    #[serde(skip_deserializing)]
    pub updated_at: Option<Datetime>,
}

pub struct TranscriptionController;

#[async_trait]
impl Controller<Transcription, NewTranscription, TranscriptionPatch> for TranscriptionController {
    async fn get(client: &Surreal<Client>, id: &SurrealId) -> Result<Option<Transcription>> {
        let mut results = client
            .query("SELECT * FROM ONLY $transcription")
            .bind(("transcription", id.clone().0))
            .await?;

        let transcription: Option<Transcription> = results.take(0)?;
        Ok(transcription)
    }

    async fn update(
        client: &Surreal<Client>,
        id: &str,
        transcription_patch: &TranscriptionPatch,
    ) -> Result<Transcription> {
        let transcription_id = extract_id(id, "transcription");
        let mut transcription_data = transcription_patch.clone();

        transcription_data.updated_at = Some(Datetime::default());

        let transcription_opt: Option<Transcription> = client
            .update(("transcription", transcription_id))
            .merge(transcription_data)
            .await?;
        let transcription =
            transcription_opt.ok_or(Error::StoreData("transcription".to_string()))?;
        Ok(transcription)
    }

    async fn create(
        client: &Surreal<Client>,
        new_transcription: &NewTranscription,
    ) -> Result<Transcription> {
        let mut transcription_data = new_transcription.clone();
        transcription_data.created_at = Some(Datetime::default());
        transcription_data.updated_at = Some(Datetime::default());

        let transcription: Option<Transcription> = client
            .create("transcription")
            .content(transcription_data)
            .await?;
        transcription.ok_or(Error::StoreData("transcription".to_string()))
    }

    async fn delete(client: &Surreal<Client>, id: &SurrealId) -> Result<()> {
        // Use backblaze connector to delete the audio file from the url
        client
            .query("DELETE transcription WHERE id = $id RETURN NONE")
            .bind(("id", id.clone().0))
            .await?;

        Ok(())
    }
}
