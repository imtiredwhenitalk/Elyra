use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::database::Database;

#[derive(Clone)]
pub struct UserService {
    database: Database,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct UserWithPassword {
    id: Uuid,
    email: String,
    display_name: String,
    password_hash: String,
    created_at: DateTime<Utc>,
}

impl UserService {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn find_by_email(
        &self,
        email: &str,
    ) -> Result<Option<(UserRecord, String)>, sqlx::Error> {
        sqlx::query_as::<_, UserWithPassword>(
            "SELECT id, email, display_name, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.database.0)
        .await
        .map(|user| {
            user.map(|value| {
                (
                    UserRecord {
                        id: value.id,
                        email: value.email,
                        display_name: value.display_name,
                        created_at: value.created_at,
                    },
                    value.password_hash,
                )
            })
        })
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<UserRecord>, sqlx::Error> {
        sqlx::query_as("SELECT id, email, display_name, created_at FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.database.0)
            .await
    }

    pub async fn find_id_by_email(&self, email: &str) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT id FROM users WHERE lower(email) = lower($1)")
            .bind(email)
            .fetch_optional(&self.database.0)
            .await
    }

    pub async fn create(
        &self,
        email: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<UserRecord, sqlx::Error> {
        sqlx::query_as(
            "INSERT INTO users (id, email, display_name, password_hash)
             VALUES ($1, $2, $3, $4)
             RETURNING id, email, display_name, created_at",
        )
        .bind(Uuid::new_v4())
        .bind(email)
        .bind(display_name)
        .bind(password_hash)
        .fetch_one(&self.database.0)
        .await
    }
}
