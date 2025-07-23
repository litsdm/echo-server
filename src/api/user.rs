use actix_web::{
    HttpMessage, HttpRequest, delete, get, put,
    web::{Data, Json},
};

use crate::{
    error::{Error, Result},
    model::{
        token::Claims,
        user::{User, UserController, UserPatch},
    },
    repo::surreal::SurrealDB,
};

#[get("/me")]
pub async fn get_user(db: Data<SurrealDB>, req: HttpRequest) -> Result<Json<User>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    let user_option = UserController::get(&db.surreal, &claims.sub).await?;

    if user_option.is_none() {
        return Err(Error::WrongCredentials);
    }

    // let backblaze = BackBlaze::new().await?;
    let user: User = user_option.unwrap();

    // user.blaze_token = Some(backblaze.authorization_token);

    Ok(Json(user))
}

#[put("")]
pub async fn update_user(
    db: Data<SurrealDB>,
    body: Json<UserPatch>,
    req: HttpRequest,
) -> Result<Json<User>> {
    let claims = req.extensions().get::<Claims>().unwrap().clone();
    let user_option = UserController::get(&db.surreal, &claims.sub).await?;

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
    let user_option = UserController::get(&db.surreal, &claims.sub).await?;

    if let Some(user) = user_option {
        UserController::delete(&db.surreal, &user.id.to_string()).await?;
        return Ok(Json(String::from("Success!")));
    }

    Err(Error::WrongCredentials)
}
