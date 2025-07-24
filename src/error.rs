use actix_web::{
    self, HttpResponse,
    http::{StatusCode, header::ContentType},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unauthorized: Wrong credentials")]
    WrongCredentials,

    #[error("Unauthorized: The provided token is not active.")]
    TokenMismatch,

    #[error("Unauthorized: There was an issue validating your token.")]
    Unauthorized,

    #[error("{0} not found")]
    NotFound(String),

    #[error("Unauthorized: Email already in use")]
    EmailInUse,

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Error creating {0}")]
    StoreData(String),

    #[error(transparent)]
    PasswordHash(#[from] argon2::password_hash::Error),

    #[error[transparent]]
    SurrealDB(#[from] surrealdb::Error),

    #[error[transparent]]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error[transparent]]
    Deserialize(#[from] serde_json::error::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::StoreData(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::SurrealDB(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::WrongCredentials => StatusCode::UNAUTHORIZED,
            Error::TokenMismatch => StatusCode::UNAUTHORIZED,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::EmailInUse => StatusCode::UNAUTHORIZED,
            Error::Jwt(_) => StatusCode::UNAUTHORIZED,
            Error::Deserialize(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::BadRequest(_) => StatusCode::BAD_REQUEST,
            Error::PasswordHash(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Reqwest(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
