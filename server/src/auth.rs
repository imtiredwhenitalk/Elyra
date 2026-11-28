use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
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
        Ok(Self::new(&secret, ttl_hours))
    }

    /// Build a service without touching process environment variables.
    ///
    /// Used by tests so parallel test cases cannot race over `ELYRA_JWT_SECRET`.
    pub(crate) fn new(secret: &str, ttl_hours: i64) -> Self {
        Self {
            secret: secret.to_string(),
            ttl_hours,
        }
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
            // Pin the accepted algorithm instead of relying on crate defaults.
            // jsonwebtoken 10 rejects alg confusion and type confusion in claims.
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims.sub)
        .map_err(|_| AuthError::InvalidToken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-only-jwt-secret-at-least-32-chars";

    fn service() -> AuthService {
        AuthService::new(SECRET, 24)
    }

    #[test]
    fn password_hashes_round_trip_and_reject_bad_input() {
        let auth = service();
        let hash = auth
            .hash_password("correct horse battery")
            .expect("hash password");
        assert_ne!(hash, "correct horse battery");
        assert!(auth.verify_password("correct horse battery", &hash).is_ok());
        assert!(matches!(
            auth.verify_password("wrong passphrase", &hash),
            Err(AuthError::InvalidCredentials)
        ));
        assert!(matches!(
            auth.verify_password("correct horse battery", "not-a-hash"),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn tokens_are_bound_to_secret_user_and_expiry() {
        let auth = service();
        let user_id = Uuid::new_v4();
        let (token, expires_at) = auth.issue(user_id).expect("issue token");
        assert_eq!(auth.verify(&token).expect("verify token"), user_id);
        assert!(expires_at > Utc::now());

        let other = AuthService::new("another-test-secret-at-least-32-chars", 24);
        assert!(matches!(other.verify(&token), Err(AuthError::InvalidToken)));

        let expired_claims = Claims {
            sub: user_id,
            exp: (Utc::now() - Duration::hours(1)).timestamp() as usize,
        };
        let expired = encode(
            &Header::default(),
            &expired_claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .expect("encode expired token");
        assert!(matches!(
            auth.verify(&expired),
            Err(AuthError::InvalidToken)
        ));

        assert!(matches!(
            auth.verify("not-a-jwt"),
            Err(AuthError::InvalidToken)
        ));
    }
}
