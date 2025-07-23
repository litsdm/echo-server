use actix_web::{
    post,
    web::{Data, Json},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    model::{
        device::{DeviceController, NewDevice},
        token::{TokenController, TokenManager, TokenResponse},
        user::{NewUser, PasswordHasher, UserController},
    },
    repo::surreal::SurrealDB,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all(deserialize = "snake_case"))]
struct UserPayload {
    pub user: NewUser,
    pub device: NewDevice,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
    pub device: NewDevice,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub struct RefreshPayload {
    #[serde(alias = "refreshToken")]
    refresh_token: String,
    #[serde(alias = "deviceId")]
    device_id: String,
}

#[post("/signup")]
pub async fn signup(db: Data<SurrealDB>, body: Json<UserPayload>) -> Result<Json<TokenResponse>> {
    let payload = body.into_inner();
    let email = payload.user.email.to_lowercase();
    let existing_user = UserController::get_by_email(&db.surreal, &email).await?;

    if existing_user.is_some() {
        return Err(Error::EmailInUse);
    }

    let user = UserController::create(&db.surreal, &payload.user).await?;

    let mut new_device = payload.device.clone();
    new_device.user_id = Some(user.clone().id.to_string());

    let device = DeviceController::create_or_update(&db.surreal, &new_device).await?;

    let token =
        TokenController::create_or_update(&db.surreal, &user, &device.id.to_string()).await?;

    Ok(Json(token))
}

#[post("/login")]
pub async fn login(db: Data<SurrealDB>, body: Json<LoginPayload>) -> Result<Json<TokenResponse>> {
    let email = body.email.to_lowercase();
    let existing_user = UserController::get_by_email(&db.surreal, &email).await?;
    let password_hasher = PasswordHasher::new();

    if existing_user.is_none() {
        return Err(Error::WrongCredentials);
    }

    let user = existing_user.unwrap();

    if user.password_hash.is_none() {
        return Err(Error::WrongCredentials);
    }

    let password = user.password_hash.as_ref().unwrap();
    password_hasher.verify(password, &body.password)?;

    let mut new_device = body.device.clone();
    new_device.user_id = Some(user.clone().id.to_string());

    let device = DeviceController::create_or_update(&db.surreal, &new_device).await?;

    let token =
        TokenController::create_or_update(&db.surreal, &user, &device.id.to_string()).await?;
    Ok(Json(token))
}

#[post("/refresh")]
pub async fn refresh(
    db: Data<SurrealDB>,
    body: Json<RefreshPayload>,
) -> Result<Json<TokenResponse>> {
    let claims = TokenManager::validate_refresh_token(&db.surreal, &body.refresh_token).await?;
    let user = UserController::get(&db.surreal, &claims.sub)
        .await?
        .unwrap();

    let token = TokenController::create_or_update(&db.surreal, &user, &body.device_id).await?;
    Ok(Json(token))
}
