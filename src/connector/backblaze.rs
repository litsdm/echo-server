use std::env;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase", serialize = "snake_case"))]
pub struct AuthorizationResponse {
    account_id: String,
    api_url: String,
    authorization_token: String,
    download_url: String,
    recommended_part_size: usize,
    absolute_minimum_part_size: usize,
    s3_api_url: String,
}

pub struct BackBlaze;

impl BackBlaze {
    pub async fn get_read_auth_token() -> Result<String> {
        let client = Client::new();
        let authorize_url =
            String::from("https://api.backblazeb2.com/b2api/v2/b2_authorize_account");

        let key_id = env::var("B2_READ_ACCESS_KEY").unwrap();
        let key = env::var("B2_READ_SECRET_KEY").unwrap();

        let auth_response = client
            .get(authorize_url)
            .basic_auth(key_id, Some(key))
            .send()
            .await?
            .json::<AuthorizationResponse>()
            .await?;

        Ok(auth_response.authorization_token)
    }
}
