use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher as APH, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};
use async_trait::async_trait;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::remote::ws::Client, sql::Datetime};
use surrealitos::{SurrealId, extract_id};

use crate::{
    error::{Error, Result},
    model::Controller,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum UserType {
    User,
    #[default]
    Guest,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct User {
    pub id: SurrealId,
    pub created_at: String,
    pub updated_at: String,
    #[serde(alias = "type")]
    pub user_type: UserType,
    pub email: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub avatar_seed: String,
    pub name: Option<String>,
    pub verified_email: Option<bool>,
    pub blaze_token: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all(deserialize = "snake_case"))]
pub struct NewUser {
    pub password: Option<String>, // This is optional because we need to remove it before commiting to DB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(alias = "avatarSeed", skip_serializing_if = "Option::is_none")]
    pub avatar_seed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_type: Option<UserType>,

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

#[async_trait]
impl Controller<User, NewUser, UserPatch> for UserController {
    async fn get(client: &Surreal<Client>, id: &SurrealId) -> Result<Option<User>> {
        let mut results = client
            .query("SELECT * FROM ONLY $user")
            .bind(("user", id.clone().0))
            .await?;

        let user: Option<User> = results.take(0)?;
        Ok(user)
    }

    async fn create(client: &Surreal<Client>, new_user: &NewUser) -> Result<User> {
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
        user_data.user_type = Some(UserType::User);

        let user: Option<User> = client.create("user").content(user_data).await?;
        user.ok_or(Error::StoreData("user".to_string()))
    }

    async fn update(client: &Surreal<Client>, id: &str, user_patch: &UserPatch) -> Result<User> {
        let user_id = extract_id(id, "user");
        let mut user_data = user_patch.clone();

        user_data.updated_at = Some(Datetime::default());

        let params = serde_json::to_value(user_data)?;

        let user_opt: Option<User> = client.update(("user", user_id)).merge(params).await?;

        let user = user_opt.ok_or(Error::StoreData("user".to_string()))?;

        Ok(user)
    }

    async fn delete(client: &Surreal<Client>, id: &SurrealId) -> Result<()> {
        client
            .query("DELETE device WHERE user_id = $user")
            .query("DELETE token WHERE user_id = $user")
            .query("DELETE $user")
            .bind(("user", id.0.clone()))
            .await?;
        Ok(())
    }
}

impl UserController {
    pub async fn get_by_email(client: &Surreal<Client>, email: &str) -> Result<Option<User>> {
        let mut results = client
            .query("SELECT * FROM user WHERE email = $email")
            .bind(("email", email.to_owned()))
            .await?;

        let user: Option<User> = results.take(0)?;
        Ok(user)
    }

    pub async fn create_guest(client: &Surreal<Client>) -> Result<User> {
        // let config = ConfigController::create_default(client).await?;
        let guest_discriminator = Alphanumeric.sample_string(&mut rand::thread_rng(), 4);

        let new_guest = NewUser {
            name: Some(format!("Guest#{guest_discriminator}")),
            user_type: Some(UserType::Guest),
            avatar_seed: Some(format!("Guest#{guest_discriminator}")),
            created_at: Some(Datetime::default()),
            updated_at: Some(Datetime::default()),
            ..Default::default()
        };

        let user: Option<User> = client.create("user").content(new_guest).await?;
        user.ok_or(Error::StoreData("user".to_string()))
    }
}
