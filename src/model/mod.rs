use async_trait::async_trait;
use surrealdb::{Surreal, engine::remote::ws::Client};
use surrealitos::SurrealId;

pub mod device;
pub mod token;
pub mod user;

#[async_trait]
pub trait Controller<T, NewT, PatchT> {
    async fn get(client: &Surreal<Client>, id: &SurrealId) -> crate::error::Result<Option<T>>;

    async fn create(client: &Surreal<Client>, new_entity: &NewT) -> crate::error::Result<T>;

    async fn update(client: &Surreal<Client>, id: &str, patch: &PatchT) -> crate::error::Result<T>;

    async fn delete(client: &Surreal<Client>, id: &SurrealId) -> crate::error::Result<()>;
}
