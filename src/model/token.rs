use std::env;

use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use surrealdb::{Surreal, engine::remote::ws::Client};
use surrealitos::{SurrealId, extract_id};

use crate::error::{Error, Result};

use super::user::User;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Token {
    pub id: SurrealId,
    #[serde(alias = "accessToken")]
    pub access_token: String,
    #[serde(alias = "refreshToken")]
    pub refresh_token: Option<String>,
    pub key: Option<String>,
    pub nonce: Option<String>,
    #[serde(alias = "userId")]
    pub user_id: String,
    #[serde(alias = "deviceId")]
    pub device_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewToken {
    #[serde(alias = "accessToken")]
    pub access_token: String,
    #[serde(alias = "refreshToken")]
    pub refresh_token: Option<String>,
    pub key: Option<String>,
    pub nonce: Option<String>,
    #[serde(alias = "userId")]
    pub user_id: String,
    #[serde(alias = "deviceId")]
    pub device_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct TokenResponse {
    pub token: String,
    #[serde(alias = "refreshToken")]
    pub refresh_token: Option<String>,
}

impl From<Token> for TokenResponse {
    fn from(token: Token) -> Self {
        TokenResponse {
            token: token.access_token,
            refresh_token: token.refresh_token,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub iat: u128,
    pub exp: i64,
    pub sub: String,
    // pub permissions: Vec<String>,
}

impl Claims {
    pub fn new(user: User, duration: Duration) -> Self {
        // TODO: get permissions from user roles
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let exp = (Utc::now() + duration).timestamp();

        Self {
            iat: timestamp,
            exp,
            sub: user.id.to_string(),
            // permissions: user.permissions,
        }
    }
}

pub struct TokenManager;

impl TokenManager {
    pub fn generate(claims: &Claims) -> Result<String> {
        let secret = env::var("JWT_SECRET").unwrap();
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )?;

        Ok(token)
    }

    // pub fn validate(token: &str) -> Result<Claims> {
    //   let secret = env::var("JWT_SECRET").unwrap();
    //   let decoded_token = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_ref()), &Validation::default())?;

    //   Ok(decoded_token.claims)
    // }

    pub async fn validate_access_token(
        client: &Surreal<Client>,
        encrypted_token: &str,
    ) -> Result<Claims> {
        let secret = env::var("JWT_SECRET").unwrap();
        let stored_token = TokenController::get_by_access_token(client, encrypted_token).await?;

        if stored_token.is_none() {
            return Err(Error::TokenMismatch);
        }

        let db_token = stored_token.unwrap();
        let key: &[u8] = &hex::decode(db_token.key.unwrap()).expect("Hex decode error");

        let nonces = db_token.nonce.unwrap();
        let nonce_parts: Vec<&str> = nonces.split(':').collect();
        let nonce: &[u8] = &hex::decode(nonce_parts[0]).expect("Hex decode error");

        let cipher = XChaCha20Poly1305::new(key.into());
        let enc_token = hex::decode(encrypted_token).expect("Error decoding token");

        let decrypted = cipher
            .decrypt(nonce.into(), enc_token.as_ref())
            .expect("Decryption error");
        let decrypted_token = std::str::from_utf8(&decrypted).unwrap();
        let decoded_token = decode::<Claims>(
            decrypted_token,
            &DecodingKey::from_secret(secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(decoded_token.claims)
    }

    pub async fn validate_refresh_token(
        client: &Surreal<Client>,
        encrypted_token: &str,
    ) -> Result<Claims> {
        let secret = env::var("JWT_SECRET").unwrap();
        let stored_token = TokenController::get_by_refresh_token(client, encrypted_token).await?;

        if stored_token.is_none() {
            return Err(Error::TokenMismatch);
        }

        let db_token = stored_token.unwrap();
        let key: &[u8] = &hex::decode(db_token.key.unwrap()).expect("Hex decode error");

        let nonces = db_token.nonce.unwrap();
        let nonce_parts: Vec<&str> = nonces.split(':').collect();
        let nonce: &[u8] = &hex::decode(nonce_parts[1]).expect("Hex decode error");

        let cipher = XChaCha20Poly1305::new(key.into());
        let enc_token = hex::decode(encrypted_token).expect("Error decoding token");

        let decrypted = cipher
            .decrypt(nonce.into(), enc_token.as_ref())
            .expect("Decryption error");
        let decrypted_token = std::str::from_utf8(&decrypted).unwrap();
        let decoded_token = decode::<Claims>(
            decrypted_token,
            &DecodingKey::from_secret(secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(decoded_token.claims)
    }
}

pub struct TokenController;

impl TokenController {
    pub async fn get_by_access_token(
        client: &Surreal<Client>,
        access_token: &str,
    ) -> Result<Option<Token>> {
        let mut results = client
            .query("SELECT * FROM token WHERE access_token = $access_token")
            .bind(("access_token", access_token.to_owned()))
            .await?;

        let token: Option<Token> = results.take(0)?;
        Ok(token)
    }

    pub async fn get_by_refresh_token(
        client: &Surreal<Client>,
        refresh_token: &str,
    ) -> Result<Option<Token>> {
        let mut results = client
            .query("SELECT * FROM token WHERE refresh_token = $refresh_token")
            .bind(("refresh_token", refresh_token.to_owned()))
            .await?;

        let token: Option<Token> = results.take(0)?;
        Ok(token)
    }

    pub async fn get_by_device(
        client: &Surreal<Client>,
        device_id: String,
    ) -> Result<Option<Token>> {
        let mut results = client
            .query("SELECT * FROM token WHERE device_id = $device_id")
            .bind(("device_id", device_id))
            .await?;

        let token: Option<Token> = results.take(0)?;
        Ok(token)
    }

    pub async fn create_or_update(
        client: &Surreal<Client>,
        user: &User,
        device_id: &str,
    ) -> Result<TokenResponse> {
        let claims = Claims::new(user.to_owned(), Duration::hours(3));
        let refresh_claims = Claims::new(user.to_owned(), Duration::days(30));

        let access_token = TokenManager::generate(&claims)?;
        let refresh_token = TokenManager::generate(&refresh_claims)?;

        let key = XChaCha20Poly1305::generate_key(&mut OsRng);
        let access_nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let refresh_nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

        let cipher = XChaCha20Poly1305::new(&key);
        let encrypted_access_token = cipher
            .encrypt(&access_nonce, access_token.as_bytes().as_ref())
            .expect("Encryption error");
        let encrypted_refresh_token = cipher
            .encrypt(&refresh_nonce, refresh_token.as_bytes().as_ref())
            .expect("Encryption error");

        let new_token = NewToken {
            access_token: hex::encode(encrypted_access_token),
            refresh_token: Some(hex::encode(encrypted_refresh_token)),
            key: Some(hex::encode(key)),
            nonce: Some(format!(
                "{}:{}",
                hex::encode(access_nonce),
                hex::encode(refresh_nonce)
            )),
            user_id: user.id.to_string(),
            device_id: device_id.to_owned(),
        };

        let stored_token = Self::get_by_device(client, device_id.to_string()).await?;

        let db_token: Token = match stored_token {
            None => {
                let token: Option<Token> = client.create("token").content(new_token).await?;
                token.ok_or(Error::StoreData("token".to_string()))?
            }
            Some(prev_token) => {
                let token_id = extract_id(&prev_token.id.to_string(), "token");
                let token: Option<Token> =
                    client.update(("token", token_id)).merge(new_token).await?;
                token.unwrap()
            }
        };

        Ok(db_token.into())
    }

    // Enable when we need email verification
    // pub async fn create_verification_token(
    //     client: &Surreal<Client>,
    //     user: &User,
    // ) -> Result<TokenResponse> {
    //     let claims = Claims::new(user.to_owned());

    //     let access_token = TokenManager::generate(&claims)?;

    //     let key = XChaCha20Poly1305::generate_key(&mut OsRng);
    //     let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

    //     let cipher = XChaCha20Poly1305::new(&key);
    //     let encrypted_access_token = cipher
    //         .encrypt(&nonce, access_token.as_bytes().as_ref())
    //         .expect("Encryption error");

    //     let new_token = NewToken {
    //         access_token: hex::encode(encrypted_access_token),
    //         refresh_token: None,
    //         key: Some(hex::encode(key)),
    //         nonce: Some(hex::encode(nonce)),
    //         user_id: user.id.to_raw(),
    //         device_id: "Verification".to_string(),
    //     };

    //     let stored_token =
    //         Self::get_verification_token_by_user(client, user.id.to_raw().as_str()).await?;

    //     let db_token: Token = match stored_token {
    //         None => {
    //             let tokens: Vec<Token> = client.create("token").content(new_token).await?;
    //             tokens
    //                 .into_iter()
    //                 .next()
    //                 .ok_or(Error::StoreData("token".to_string()))?
    //         }
    //         Some(prev_token) => {
    //             let token_id = extract_id(&prev_token.id.to_raw(), "token");
    //             let token: Option<Token> =
    //                 client.update(("token", token_id)).merge(new_token).await?;
    //             token.unwrap()
    //         }
    //     };

    //     Ok(db_token.into())
    // }

    // pub async fn get_verification_token_by_user(
    //     client: &Surreal<Client>,
    //     user_id: &str,
    // ) -> Result<Option<Token>> {
    //     let mut results = client
    //         .query("SELECT * FROM token WHERE user_id = $user_id AND device_id = $device_id")
    //         .bind(("user_id", user_id))
    //         .bind(("device_id", "Verification"))
    //         .await?;

    //     let token: Option<Token> = results.take(0)?;
    //     Ok(token)
    // }

    // pub async fn delete_verification_token_by_user(
    //     client: &Surreal<Client>,
    //     user_id: &str,
    // ) -> Result<()> {
    //     client
    //         .query("DELETE token WHERE user_id = $user_id AND device_id = $device_id")
    //         .bind(("user_id", user_id))
    //         .bind(("device_id", "Verification"))
    //         .await?;
    //     Ok(())
    // }
}
