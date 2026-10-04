use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthService {
    secret: String,
    ttl_hours: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("ELYRA_JWT_SECRET is not configured")]
    MissingSecret,
    #[error("password hashing failed")]
    Hash,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid access token")]
    InvalidToken,
}

impl AuthService {
    pub fn from_env() -> Result<Self, AuthError> {
        let secret = env::var("ELYRA_JWT_SECRET").map_err(|_| AuthError::MissingSecret)?;
        if secret.len() < 32 {
            return Err(AuthError::MissingSecret);
        }
        let ttl_hours = env::var("ELYRA_ACCESS_TOKEN_TTL_HOURS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(24);
        Ok(Self { secret, ttl_hours })
    }

    pub fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| AuthError::Hash)
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<(), AuthError> {
        let parsed = PasswordHash::new(hash).map_err(|_| AuthError::InvalidCredentials)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| AuthError::InvalidCredentials)
    }

    pub fn issue(&self, user_id: Uuid) -> Result<(String, chrono::DateTime<Utc>), AuthError> {
        let expires_at = Utc::now() + Duration::hours(self.ttl_hours);
        let claims = Claims {
            sub: user_id,
            exp: expires_at.timestamp() as usize,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map(|token| (token, expires_at))
        .map_err(|_| AuthError::InvalidToken)
    }

    pub fn verify(&self, token: &str) -> Result<Uuid, AuthError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims.sub)
        .map_err(|_| AuthError::InvalidToken)
    }
}
