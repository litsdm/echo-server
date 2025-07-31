use std::env;

pub mod auth;
pub mod device;
pub mod storage;
pub mod transcription;
pub mod user;

pub fn get_default_webhook_base() -> String {
    let project_env = env::var("PROJECT_ENV").unwrap_or(String::from("development"));
    let base_url = match project_env.as_str() {
        "prod" => "https://echo-server.fly.dev".to_string(),
        "development" => "https://8a40d8967902.ngrok-free.app".to_string(),
        _ => "".to_string(),
    };

    format!("{base_url}/webhook")
}

pub fn make_default_webhook_url(tool_type: &str) -> String {
    let base_url = get_default_webhook_base();

    format!("{base_url}/{tool_type}/status")
}
