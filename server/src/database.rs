use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use thiserror::Error;

#[derive(Clone)]
pub struct Database(pub PgPool);

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("DATABASE_URL is not configured")]
    MissingUrl,
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl Database {
    pub async fn connect_from_env() -> Result<Self, DatabaseError> {
        let url = env::var("DATABASE_URL").map_err(|_| DatabaseError::MissingUrl)?;
        Ok(Self(
            PgPoolOptions::new()
                .max_connections(10)
                .connect(&url)
                .await?,
        ))
    }

    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS sync_cursors (
                user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                cursor TEXT,
                synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS notifications (
                id UUID PRIMARY KEY,
                sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                is_read BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );",
        )
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
