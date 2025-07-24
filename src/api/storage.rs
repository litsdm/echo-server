use std::env;

use actix_web::{
    get,
    web::{Json, Query},
};
use serde::{Deserialize, Serialize};

use rusoto_core::{
    Region,
    credential::{AwsCredentials, ChainProvider, ProvideAwsCredentials},
};
use rusoto_s3::{
    GetObjectRequest, PutObjectRequest,
    util::{PreSignedRequest, PreSignedRequestOption},
};

use crate::error::Result;

struct AwsConfig {
    region: Region,
    credentials: AwsCredentials,
    options: PreSignedRequestOption,
}

#[derive(Deserialize, Serialize)]
pub struct SignParams {
    key: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct PresignedUrlResponse {
    signed_url: String,
    url: String,
}

async fn aws_config() -> AwsConfig {
    let provider = ChainProvider::new();
    let credentials = provider.credentials().await.unwrap();
    let region = Region::Custom {
        name: "us-west-004".to_string(),
        endpoint: "s3.us-west-004.backblazeb2.com".to_string(),
    };

    let options = PreSignedRequestOption {
        expires_in: std::time::Duration::from_secs(300),
    };

    AwsConfig {
        region,
        credentials,
        options,
    }
}

#[get("/sign/put")]
pub async fn presign_put(query: Query<SignParams>) -> Result<Json<PresignedUrlResponse>> {
    let payload = query.into_inner();
    let bucket = env::var("BUCKET").unwrap();
    let config = aws_config().await;
    let req = PutObjectRequest {
        bucket: bucket.to_string(),
        key: payload.key.clone(),
        ..Default::default()
    };

    let signed_url = req.get_presigned_url(&config.region, &config.credentials, &config.options);
    let url = format!(
        "https://f004.backblazeb2.com/file/{bucket}/{}",
        &payload.key
    );

    let res = PresignedUrlResponse { signed_url, url };

    Ok(Json(res))
}

#[get("/sign/get")]
pub async fn presign_get(query: Query<SignParams>) -> Result<Json<PresignedUrlResponse>> {
    let payload = query.into_inner();
    let bucket = env::var("BUCKET").unwrap();
    let config = aws_config().await;
    let req = GetObjectRequest {
        bucket: bucket.to_string(),
        key: payload.key.clone(),
        ..Default::default()
    };

    let signed_url = req.get_presigned_url(&config.region, &config.credentials, &config.options);
    let url = format!(
        "https://f004.backblazeb2.com/file/{bucket}/{}",
        &payload.key
    );

    let res = PresignedUrlResponse { signed_url, url };

    Ok(Json(res))
}
