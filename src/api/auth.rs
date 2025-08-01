use std::str::FromStr;

use actix_web::{
    HttpMessage, HttpRequest, get, post,
    web::{Data, Json, Path},
};
use serde::{Deserialize, Serialize};
use surrealitos::SurrealId;

use crate::{
    error::{Error, Result},
    model::{
        Controller,
        device::{DeviceController, DevicePatch, NewDevice},
        token::{Claims, TokenController, TokenManager, TokenResponse},
        user::{NewUser, PasswordHasher, UserController},
    },
    repo::surreal::SurrealDB,
};

#[derive(Serialize, Deserialize)]
struct UserPayload {
    pub user: NewUser,
    pub device: NewDevice,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GuestPayload {
    pub device: NewDevice,
}

#[derive(Serialize, Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
    pub device: NewDevice,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshPayload {
    #[serde(alias = "refreshToken")]
    refresh_token: String,
    #[serde(alias = "deviceId")]
    device_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct CheckEmailResponse {
    exists: bool,
}

fn get_new_user_email(new_user: &NewUser) -> Result<String> {
    if new_user.email.is_none() {
        return Err(Error::BadRequest("Email is required".to_string()));
    }

    Ok(new_user.clone().email.unwrap().to_lowercase())
}

#[post("/signup")]
pub async fn signup(db: Data<SurrealDB>, body: Json<UserPayload>) -> Result<Json<TokenResponse>> {
    let payload = body.into_inner();
    let email = get_new_user_email(&payload.user)?;
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
    let id = SurrealId::from_str(&claims.sub)?;
    let user = UserController::get(&db.surreal, &id).await?.unwrap();

    let token = TokenController::create_or_update(&db.surreal, &user, &body.device_id).await?;
    Ok(Json(token))
}

#[post("/guest")]
pub async fn guest(db: Data<SurrealDB>, body: Json<GuestPayload>) -> Result<Json<TokenResponse>> {
    let payload = body.into_inner();
    let device = DeviceController::create_or_update(&db.surreal, &payload.device).await?;

    let guest = match device.get_guest(&db.surreal).await? {
        Some(g) => g,
        None => UserController::create_guest(&db.surreal).await?,
    };

    let guest_id = guest.clone().id;
    let device_patch = DevicePatch {
        user_id: Some(guest_id.to_string()),
        guest_id: Some(guest_id.to_string()),
        ..Default::default()
    };

    DeviceController::update(&db.surreal, &device.id.to_string(), &device_patch).await?;

    let token =
        TokenController::create_or_update(&db.surreal, &guest, &device.id.to_string()).await?;
    Ok(Json(token))
}

#[get("/email-exists/{email}")]
pub async fn check_email_exists(
    db: Data<SurrealDB>,
    path: Path<String>,
) -> Result<Json<CheckEmailResponse>> {
    let email = path.into_inner().to_lowercase();
    let existing_user = UserController::get_by_email(&db.surreal, &email).await?;
    Ok(Json(CheckEmailResponse {
        exists: existing_user.is_some(),
    }))
}

#[post("/validate")]
pub async fn validate_token(req: HttpRequest) -> Result<Json<Claims>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    Ok(Json(claims))
}
