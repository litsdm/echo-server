mod api;
mod connector;
mod error;
mod model;
mod repo;

use actix_cors::Cors;
use actix_web::{
    App, Error, HttpMessage, HttpServer,
    dev::ServiceRequest,
    middleware::Logger,
    web::{Data, scope},
};
// use actix_web_grants::permissions::AttachPermissions;
use actix_web_httpauth::{extractors::bearer::BearerAuth, middleware::HttpAuthentication};

use api::{
    auth::{login, signup},
    user::{delete_user, get_user, update_user},
};
use dotenv::dotenv;
use model::token::TokenManager;
use repo::surreal::SurrealDB;

use crate::api::{
    auth::{check_email_exists, guest, refresh, validate_token},
    storage::{presign_get, presign_put},
    transcription::{diarize_webhook, get_user_transcriptions, transcribe, transcribe_raw_only},
};

async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let db = req.app_data::<Data<SurrealDB>>().unwrap();
    let result = TokenManager::validate_access_token(&db.surreal, credentials.token()).await;

    match result {
        Ok(claims) => {
            // req.attach(claims.permissions.clone());
            req.extensions_mut().insert(claims);
            Ok(req)
        }
        Err(_) => Err((crate::error::Error::Unauthorized.into(), req)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    unsafe {
        std::env::set_var("RUST_LOG", "debug");
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    env_logger::init();

    let surreal = SurrealDB::init()
        .await
        .expect("Error connecting to SurrealDB");
    let surreal_data = Data::new(surreal);

    HttpServer::new(move || {
        let auth = HttpAuthentication::bearer(validator);
        let logger = Logger::default();
        App::new()
            .app_data(Data::clone(&surreal_data))
            .wrap(logger)
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![
                        actix_web::http::header::AUTHORIZATION,
                        actix_web::http::header::ACCEPT,
                        actix_web::http::header::CONTENT_TYPE,
                    ])
                    .max_age(3600),
            )
            .service(
                scope("/auth")
                    .service(login)
                    .service(signup)
                    .service(refresh)
                    .service(guest)
                    .service(check_email_exists),
            )
            .service(
                scope("/api")
                    .wrap(auth)
                    .service(scope("/auth").service(validate_token))
                    .service(
                        scope("/user")
                            .service(get_user)
                            .service(update_user)
                            .service(delete_user),
                    )
                    .service(scope("/storage").service(presign_put).service(presign_get))
                    .service(
                        scope("/transcription")
                            .service(get_user_transcriptions)
                            .service(transcribe_raw_only)
                            .service(transcribe),
                    ),
            )
            .service(scope("webhook").service(diarize_webhook))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
