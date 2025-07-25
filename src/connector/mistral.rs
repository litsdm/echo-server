use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Deserialize, Serialize)]
pub struct Segment {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Deserialize, Serialize)]
pub struct Usage {
    pub completion_tokens: usize,
    pub prompt_audio_seconds: usize,
    pub prompt_tokens: usize,
    pub total_tokens: usize,
}

#[derive(Deserialize, Serialize)]
pub struct TranscriptionResponse {
    pub text: String,
    pub language: String,
    pub model: String,
    pub segments: Option<Vec<Segment>>,
    pub usage: Option<Usage>,
}

pub struct Mistral {
    client: Client,
    base_url: String,
}

impl Mistral {
    pub fn new() -> Self {
        let client = Client::new();

        Mistral {
            client,
            base_url: "https://api.mistral.ai/v1".to_string(),
        }
    }

    pub async fn transcribe(&self, file_url: &str) -> Result<TranscriptionResponse> {
        let url = format!("{}/audio/transcriptions", self.base_url);
        let form = vec![("file_url", file_url), ("model", "voxtral-mini-2507")];

        let mistral_key = env::var("MISTRAL_API_KEY").unwrap();

        let response = self
            .client
            .post(url)
            .header("x-api-key", mistral_key)
            .form(&form)
            .send()
            .await?
            .json::<TranscriptionResponse>()
            .await?;

        Ok(response)
    }
}
