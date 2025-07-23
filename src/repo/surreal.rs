use std::env;

use surrealdb::engine::remote::ws::{Client, Wss};
use surrealdb::opt::auth::Root;
use surrealdb::{Result, Surreal};

#[derive(Debug, Clone)]
pub struct SurrealDB {
    pub surreal: Surreal<Client>,
}

impl SurrealDB {
    pub async fn init() -> Result<Self> {
        let client = Surreal::new::<Wss>("your-db.fly.dev").await?;

        let password =
            env::var("SURREAL_PASS").unwrap_or_else(|_| String::from("noaccessforuhoneynonono"));

        client
            .signin(Root {
                username: "dbuser",
                password: password.as_str(),
            })
            .await?;

        let db_env = env::var("PROJECT_ENV").unwrap_or_else(|_| String::from("development"));
        let db_name = if db_env == "prod" { "prod" } else { "staging" };

        client.use_ns("dbns").use_db(db_name).await?;

        Ok(SurrealDB { surreal: client })
    }
}
