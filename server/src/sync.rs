use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

use crate::database::Database;

#[derive(Clone)]
pub struct SyncService {
    database: Database,
}

impl SyncService {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn cursor(
        &self,
        user_id: Uuid,
    ) -> Result<Option<(Option<String>, DateTime<Utc>)>, sqlx::Error> {
        sqlx::query("SELECT cursor, synced_at FROM sync_cursors WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.database.0)
            .await
            .map(|row| row.map(|value| (value.get("cursor"), value.get("synced_at"))))
    }

    pub async fn update_cursor(
        &self,
        user_id: Uuid,
        cursor: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO sync_cursors (user_id, cursor, synced_at) VALUES ($1, $2, NOW())
             ON CONFLICT (user_id) DO UPDATE SET cursor = EXCLUDED.cursor, synced_at = NOW()",
        )
        .bind(user_id)
        .bind(cursor)
        .execute(&self.database.0)
        .await?;
        Ok(())
    }
}
