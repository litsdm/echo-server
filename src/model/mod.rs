use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::remote::ws::Client};
use surrealitos::SurrealId;

pub mod device;
pub mod token;
pub mod transcription;
pub mod user;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    Google,
    Ollama, // For handling on desktop
}

#[async_trait]
pub trait Controller<T, NewT, PatchT> {
    async fn get(client: &Surreal<Client>, id: &SurrealId) -> crate::error::Result<Option<T>>;

    async fn create(client: &Surreal<Client>, new_entity: &NewT) -> crate::error::Result<T>;

    async fn update(client: &Surreal<Client>, id: &str, patch: &PatchT) -> crate::error::Result<T>;

    async fn delete(client: &Surreal<Client>, id: &SurrealId) -> crate::error::Result<()>;
}
