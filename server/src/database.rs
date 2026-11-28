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
        // Create the legacy table first so upgrades from exact-case email
        // storage can be repaired before a case-insensitive unique index is
        // added. Fresh installs take the same path and end up normalized.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );",
        )
        .execute(&self.0)
        .await?;

        // Old releases allowed `User@example.com` and `user@example.com` as
        // separate rows. Detect them before normalizing so the update cannot
        // hit the exact-case unique constraint first.
        if let Some(email) = sqlx::query_scalar::<_, String>(
            "SELECT lower(trim(email)) FROM users
             GROUP BY lower(trim(email)) HAVING COUNT(*) > 1 ORDER BY 1 LIMIT 1",
        )
        .fetch_optional(&self.0)
        .await?
        {
            return Err(sqlx::Error::Protocol(format!(
                "duplicate normalized email {email:?}; merge or delete duplicate users before starting Elyra"
            )));
        }

        sqlx::query(
            "UPDATE users SET email = lower(trim(email))
             WHERE email <> lower(trim(email))",
        )
        .execute(&self.0)
        .await?;

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_lower ON users (lower(email));
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
