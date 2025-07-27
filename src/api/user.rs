use std::str::FromStr;

use actix_web::{
    HttpMessage, HttpRequest, delete, get, put,
    web::{Data, Json},
};
use surrealitos::SurrealId;

use crate::{
    connector::backblaze::BackBlaze,
    error::{Error, Result},
    model::{
        Controller,
        token::Claims,
        user::{User, UserController, UserPatch},
    },
    repo::surreal::SurrealDB,
};

#[get("/me")]
pub async fn get_user(db: Data<SurrealDB>, req: HttpRequest) -> Result<Json<User>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    let id = SurrealId::from_str(&claims.sub)?;
    let user_option = UserController::get(&db.surreal, &id).await?;

    if user_option.is_none() {
        return Err(Error::WrongCredentials);
    }

    let read_token = BackBlaze::get_read_auth_token().await?;
    let mut user: User = user_option.unwrap();
    user.blaze_token = Some(read_token);

    Ok(Json(user))
}

#[put("")]
pub async fn update_user(
    db: Data<SurrealDB>,
    body: Json<UserPatch>,
    req: HttpRequest,
) -> Result<Json<User>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    let id = SurrealId::from_str(&claims.sub)?;
    let user_option = UserController::get(&db.surreal, &id).await?;

    if user_option.is_none() {
        return Err(Error::WrongCredentials);
    }

    let user = user_option.unwrap();

    let updated_user =
        UserController::update(&db.surreal, &user.id.to_string(), &body.into_inner()).await?;

    Ok(Json(updated_user))
}

#[delete("")]
pub async fn delete_user(db: Data<SurrealDB>, req: HttpRequest) -> Result<Json<String>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    let id = SurrealId::from_str(&claims.sub)?;
    let user_option = UserController::get(&db.surreal, &id).await?;

    if let Some(user) = user_option {
        UserController::delete(&db.surreal, &user.id).await?;
        return Ok(Json(String::from("Success!")));
    }

    Err(Error::WrongCredentials)
}
