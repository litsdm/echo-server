use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Serialize, Deserialize)]
pub struct UpdatePayload<D> {
    data: D,
}

#[derive(Debug, Clone)]
pub struct Reverb {
    client: Client,
    base_url: String,
}

impl Reverb {
    pub fn new() -> Self {
        let project_env = env::var("PROJECT_ENV").unwrap_or(String::from("development"));
        let base_url = match project_env.as_str() {
            "prod" => "https://reverb.fly.dev",
            _ => "http://localhost:4000",
        };
        Reverb {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn notify_update<D>(&self, endpoint: &str, data: D) -> Result<()>
    where
        D: Serialize,
    {
        let url = format!("{}/api/{endpoint}", self.base_url);

        let payload = UpdatePayload { data };

        self.client
            .post(url)
            .json::<UpdatePayload<D>>(&payload)
            .send()
            .await?;

        Ok(())
    }
}
