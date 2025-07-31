use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use surrealitos::SurrealId;

use crate::{connector::HttpMethod, error::Result, model::transcription::Segment};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BaseParameters {
    pub webhook_url: String,
    pub job_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiarizationInput {
    pub audio: String,
    pub segments: Vec<Segment>,
    #[serde(flatten)]
    pub base: BaseParameters,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    #[serde(alias = "queued")]
    Starting,
    Processing,
    #[serde(alias = "finished")]
    Success,
    Error,
    Cancelled,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResultOutput<D> {
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<D>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<SurrealId>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolAsyncIO {
    pub call_id: String,
}

pub struct ModalAI {
    client: Client,
    pub base_url: String,
}

impl ModalAI {
    pub fn new() -> Self {
        let project_env = env::var("PROJECT_ENV").unwrap_or(String::from("development"));
        let base_url = match project_env.as_str() {
            "prod" => "https://litsdm--orestiad-main.modal.run".to_string(),
            _ => "https://litsdm--orestiad-main-dev.modal.run".to_string(),
        };

        ModalAI {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn run<I, O>(&self, method: HttpMethod, endpoint: &str, input: &I) -> Result<O>
    where
        I: Serialize,
        O: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/{}", self.base_url, endpoint);

        let request = match method {
            HttpMethod::Get => self.client.get(&url).query(input),
            HttpMethod::Post => self.client.post(&url).json(input),
            HttpMethod::Put => self.client.put(&url).json(input),
            HttpMethod::Delete => self.client.delete(&url).json(input),
        };

        let response = request.send().await?;

        let data = response.json::<O>().await?;

        Ok(data)
    }

    pub async fn result<O>(&self, call_id: &str) -> Result<O>
    where
        O: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/result/{call_id}", self.base_url);

        let response = self.client.get(url).send().await?.json::<O>().await?;

        Ok(response)
    }
}
