use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use surrealitos::SurrealId;

use crate::{
    connector::{HttpMethod, mistral::Segment},
    error::Result,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiarizationInput {
    pub audio: String,
    pub segments: Vec<Segment>,
    pub webhook_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum Gpu {
    #[default]
    T4,
    L4,
    A10G,
    A100,
    A100_80,
    H100,
    H200,
    B200,
    Cpu,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContainerTime {
    pub gpu: Gpu,
    pub time: f64,
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

fn cost_per_second_by_gpu(gpu: Gpu) -> f64 {
    match gpu {
        Gpu::B200 => 0.001736,
        Gpu::H200 => 0.001261,
        Gpu::H100 => 0.001097,
        Gpu::A100_80 => 0.000694,
        Gpu::A100 => 0.000583,
        Gpu::A10G => 0.000306,
        Gpu::L4 => 0.000222,
        Gpu::T4 => 0.000164,
        Gpu::Cpu => 0.0000131,
    }
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
