//! PostgreSQL integration tests for the schema owned by `elyra-server`.
//!
//! These tests never run by default so `cargo test -p elyra-server` works on a
//! machine without Docker or PostgreSQL. Point them at a disposable database:
//!
//! ```sh
//! ELYRA_TEST_DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra_test \
//!   cargo test -p elyra-server --test database_integration
//! ```
//!
//! `DATABASE_URL` is used as a fallback. If it is unreachable the tests print a
//! skip message. An explicit `ELYRA_TEST_DATABASE_URL` must work, so CI can opt
//! in without silently skipping coverage.

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, time::Duration};
use uuid::Uuid;

// Shared with the in-crate API tests so parallel test binaries cannot race on
// CREATE TABLE IF NOT EXISTS.
const MIGRATION_LOCK: i64 = 0x454C5952415F5445;

// Mirrors Database::migrate in server/src/database.rs; keep the two in sync.
const MIGRATION: &str = "CREATE TABLE IF NOT EXISTS users (
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
    );";

fn database_url() -> Option<(String, bool)> {
    for (name, explicit) in [("ELYRA_TEST_DATABASE_URL", true), ("DATABASE_URL", false)] {
        if let Ok(url) = env::var(name) {
            if !url.trim().is_empty() {
                return Some((url, explicit));
            }
        }
    }
    None
}

async fn connect() -> Option<PgPool> {
    let Some((url, explicit)) = database_url() else {
        eprintln!("skipping database integration test: set ELYRA_TEST_DATABASE_URL to enable it");
        return None;
    };
    match PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(Duration::from_secs(2))
        .connect(&url)
        .await
    {
        Ok(pool) => Some(pool),
        Err(error) if explicit => panic!("ELYRA_TEST_DATABASE_URL is unusable: {error}"),
        Err(error) => {
            eprintln!("skipping database integration test: {error}");
            None
        }
    }
}

async fn migrate(pool: &PgPool) {
    let mut connection = pool.acquire().await.expect("acquire connection");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(MIGRATION_LOCK)
        .execute(&mut *connection)
        .await
        .expect("acquire migration lock");
    let applied = sqlx::query(MIGRATION).execute(pool).await;
    let unlocked = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(MIGRATION_LOCK)
        .execute(&mut *connection)
        .await;
    applied.expect("apply schema");
    unlocked.expect("release migration lock");
}

async fn insert_user(pool: &PgPool, email: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash)
         VALUES ($1, $2, 'Test User', 'test-hash')",
    )
    .bind(id)
    .bind(email)
    .execute(pool)
    .await
    .expect("insert user");
    id
}

async fn cleanup_user(pool: &PgPool, id: Uuid) {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .expect("cleanup user");
}

#[tokio::test]
async fn migrations_are_idempotent_and_email_is_unique() {
    let Some(pool) = connect().await else {
        return;
    };
    migrate(&pool).await;
    migrate(&pool).await;

    let email = format!("{}@example.test", Uuid::new_v4());
    let id = insert_user(&pool, &email).await;
    let stored: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email.as_str())
        .fetch_one(&pool)
        .await
        .expect("read user");
    assert_eq!(stored, id);

    let duplicate = sqlx::query(
        "INSERT INTO users (id, email, display_name, password_hash)
         VALUES ($1, $2, 'Other', 'test-hash')",
    )
    .bind(Uuid::new_v4())
    .bind(email.as_str())
    .execute(&pool)
    .await;
    assert!(duplicate.is_err(), "duplicate email must be rejected");

    cleanup_user(&pool, id).await;
    pool.close().await;
}

#[tokio::test]
async fn cursor_upserts_and_notifications_follow_their_users() {
    let Some(pool) = connect().await else {
        return;
    };
    migrate(&pool).await;

    let sender = insert_user(&pool, &format!("{}@example.test", Uuid::new_v4())).await;
    let recipient = insert_user(&pool, &format!("{}@example.test", Uuid::new_v4())).await;

    for cursor in ["first", "second"] {
        sqlx::query(
            "INSERT INTO sync_cursors (user_id, cursor, synced_at) VALUES ($1, $2, NOW())
             ON CONFLICT (user_id) DO UPDATE SET cursor = EXCLUDED.cursor, synced_at = NOW()",
        )
        .bind(recipient)
        .bind(cursor)
        .execute(&pool)
        .await
        .expect("upsert cursor");
    }
    let stored: String = sqlx::query_scalar("SELECT cursor FROM sync_cursors WHERE user_id = $1")
        .bind(recipient)
        .fetch_one(&pool)
        .await
        .expect("read cursor");
    assert_eq!(stored, "second");

    let notification_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO notifications (id, sender_id, recipient_id, title, body)
         VALUES ($1, $2, $3, 'Hello', 'Body')",
    )
    .bind(notification_id)
    .bind(sender)
    .bind(recipient)
    .execute(&pool)
    .await
    .expect("insert notification");

    let unread: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications
         WHERE recipient_id = $1 AND is_read = FALSE",
    )
    .bind(recipient)
    .fetch_one(&pool)
    .await
    .expect("count unread");
    assert_eq!(unread, 1);

    sqlx::query("UPDATE notifications SET is_read = TRUE WHERE id = $1 AND recipient_id = $2")
        .bind(notification_id)
        .bind(sender)
        .execute(&pool)
        .await
        .expect("mark read (wrong owner)");
    let still_unread: bool = sqlx::query_scalar("SELECT is_read FROM notifications WHERE id = $1")
        .bind(notification_id)
        .fetch_one(&pool)
        .await
        .expect("read notification");
    assert!(!still_unread, "another user must not be able to mark it read");

    sqlx::query("UPDATE notifications SET is_read = TRUE WHERE id = $1 AND recipient_id = $2")
        .bind(notification_id)
        .bind(recipient)
        .execute(&pool)
        .await
        .expect("mark read (owner)");
    let read: bool = sqlx::query_scalar("SELECT is_read FROM notifications WHERE id = $1")
        .bind(notification_id)
        .fetch_one(&pool)
        .await
        .expect("read notification");
    assert!(read);

    cleanup_user(&pool, recipient).await;
    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE id = $1")
        .bind(notification_id)
        .fetch_one(&pool)
        .await
        .expect("count notification");
    assert_eq!(remaining, 0, "notifications cascade with their user");
    let cursor_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sync_cursors WHERE user_id = $1")
            .bind(recipient)
            .fetch_one(&pool)
            .await
            .expect("count cursor");
    assert_eq!(cursor_rows, 0, "cursors cascade with their user");

    cleanup_user(&pool, sender).await;
    pool.close().await;
}
