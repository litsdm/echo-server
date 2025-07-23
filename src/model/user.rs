use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher as APH, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};
use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::remote::ws::Client, sql::Datetime};
use surrealitos::{SurrealId, extract_id};

use crate::error::{Error, Result};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct User {
    pub id: SurrealId,
    pub created_at: String,
    pub updated_at: String,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    #[serde(alias = "avatarSeed")]
    pub avatar_seed: String,
    pub name: Option<String>,
    #[serde(alias = "verifiedEmail")]
    pub verified_email: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(deserialize = "snake_case"))]
pub struct NewUser {
    pub email: String,
    pub password: Option<String>, // This is optional because we need to remove it before commiting to DB
    #[serde(alias = "avatarSeed", skip_serializing_if = "Option::is_none")]
    pub avatar_seed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_deserializing)]
    pub password_hash: Option<String>,
    #[serde(skip_deserializing)]
    pub created_at: Option<Datetime>,
    #[serde(skip_deserializing)]
    pub updated_at: Option<Datetime>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct UserPatch {
    pub name: Option<String>,
    #[serde(alias = "avatarSeed", skip_serializing_if = "Option::is_none")]
    pub avatar_seed: Option<String>,
    #[serde(alias = "verifiedEmail", skip_serializing_if = "Option::is_none")]
    pub verified_email: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,

    #[serde(skip_deserializing)]
    pub updated_at: Option<Datetime>,
}

pub struct PasswordHasher<'a> {
    argon2: Argon2<'a>,
}

impl PasswordHasher<'_> {
    pub fn new() -> Self {
        PasswordHasher {
            argon2: Argon2::default(),
        }
    }

    pub fn derive(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);

        Ok(self
            .argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string())
    }

    pub fn verify(&self, password_hash: &str, attempted_password: &str) -> Result<()> {
        let parsed_hash = PasswordHash::new(password_hash)?;
        if self
            .argon2
            .verify_password(attempted_password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            Ok(())
        } else {
            Err(Error::WrongCredentials)
        }
    }
}

pub struct UserController;

impl UserController {
    pub async fn get(client: &Surreal<Client>, id: &str) -> Result<Option<User>> {
        let user_id = extract_id(id, "user");
        let user: Option<User> = client.select(("user", user_id)).await?;

        Ok(user)
    }

    pub async fn get_by_email(client: &Surreal<Client>, email: &str) -> Result<Option<User>> {
        let mut results = client
            .query("SELECT * FROM user WHERE email = $email")
            .bind(("email", email.to_owned()))
            .await?;

        let user: Option<User> = results.take(0)?;
        Ok(user)
    }

    pub async fn create(client: &Surreal<Client>, new_user: &NewUser) -> Result<User> {
        let password_hasher = PasswordHasher::new();
        let mut user_data = new_user.clone();

        let password = user_data
            .password
            .as_ref()
            .ok_or(Error::BadRequest("password is required".to_string()))?;
        let password_hash = password_hasher.derive(password)?;
        user_data.password = None;
        user_data.password_hash = Some(password_hash);
        user_data.created_at = Some(Datetime::default());
        user_data.updated_at = Some(Datetime::default());

        let user: Option<User> = client.create("user").content(user_data).await?;
        user.ok_or(Error::StoreData("user".to_string()))
    }

    pub async fn update(
        client: &Surreal<Client>,
        id: &str,
        user_patch: &UserPatch,
    ) -> Result<User> {
        let user_id = extract_id(id, "user");
        let mut user_data = user_patch.clone();

        user_data.updated_at = Some(Datetime::default());

        let params = serde_json::to_value(user_data)?;

        let user_opt: Option<User> = client.update(("user", user_id)).merge(params).await?;

        let user = user_opt.ok_or(Error::StoreData("user".to_string()))?;

        Ok(user)
    }

    pub async fn delete(client: &Surreal<Client>, id: &str) -> Result<()> {
        let user_id = id.to_owned();
        // we use format! macro instead of .bind because with .bind id is taken as a Thing and it does not match any devices or tokens.
        client
            .query(format!("DELETE device WHERE user_id = '{}'", user_id))
            .query(format!("DELETE token WHERE user_id = '{}'", user_id))
            .await?;
        let user_id = extract_id(id, "user");
        let _user: Option<User> = client.delete(("user", user_id)).await?;
        Ok(())
    }
}
