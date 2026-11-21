use axum::{
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::{auth::AuthError, mailer::MailError, users::UserRecord, AppState};

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/users/me", get(me))
        .route("/v1/sync/cursor", get(sync_cursor).put(update_cursor))
        .route(
            "/v1/notifications",
            get(list_notifications).post(create_notification),
        )
        .route("/v1/notifications/:id/read", patch(mark_notification_read))
        .route("/v1/mail/status", get(mail_status))
        .route("/v1/mail/send", post(send_mail))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "elyra-server" }))
}

#[derive(Debug, Deserialize)]
struct Credentials {
    email: String,
    password: String,
    #[serde(default = "default_display_name")]
    display_name: String,
}

fn default_display_name() -> String {
    "Elyra user".into()
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    access_token: String,
    expires_at: String,
    user: UserRecord,
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Credentials>,
) -> Result<Json<SessionResponse>, ApiError> {
    validate_credentials(&input)?;
    if state
        .users
        .find_by_email(&input.email)
        .await
        .map_err(ApiError::Database)?
        .is_some()
    {
        return Err(ApiError::Conflict("email is already registered".into()));
    }
    let hash = state
        .auth
        .hash_password(&input.password)
        .map_err(ApiError::Auth)?;
    let user = state
        .users
        .create(&input.email, &input.display_name, &hash)
        .await
        .map_err(ApiError::Database)?;
    session_response(&state, user)
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(input): Json<Credentials>,
) -> Result<Json<SessionResponse>, ApiError> {
    validate_credentials(&input)?;
    let Some((user, hash)) = state
        .users
        .find_by_email(&input.email)
        .await
        .map_err(ApiError::Database)?
    else {
        return Err(ApiError::Auth(AuthError::InvalidCredentials));
    };
    state
        .auth
        .verify_password(&input.password, &hash)
        .map_err(ApiError::Auth)?;
    session_response(&state, user)
}

fn session_response(state: &AppState, user: UserRecord) -> Result<Json<SessionResponse>, ApiError> {
    let (access_token, expires_at) = state.auth.issue(user.id).map_err(ApiError::Auth)?;
    Ok(Json(SessionResponse {
        access_token,
        expires_at: expires_at.to_rfc3339(),
        user,
    }))
}

async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<UserRecord>, ApiError> {
    let user_id = authenticated_user(&state, &headers)?;
    state
        .users
        .find_by_id(user_id)
        .await
        .map_err(ApiError::Database)?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

#[derive(Debug, Serialize)]
struct CursorResponse {
    cursor: Option<String>,
    synced_at: Option<String>,
}

async fn sync_cursor(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<CursorResponse>, ApiError> {
    let user_id = authenticated_user(&state, &headers)?;
    let cursor = state
        .sync
        .cursor(user_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(
        cursor
            .map(|(value, time)| CursorResponse {
                cursor: value,
                synced_at: Some(time.to_rfc3339()),
            })
            .unwrap_or(CursorResponse {
                cursor: None,
                synced_at: None,
            }),
    ))
}

#[derive(Debug, Deserialize)]
struct CursorInput {
    cursor: Option<String>,
}

async fn update_cursor(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CursorInput>,
) -> Result<StatusCode, ApiError> {
    let user_id = authenticated_user(&state, &headers)?;
    state
        .sync
        .update_cursor(user_id, input.cursor.as_deref())
        .await
        .map_err(ApiError::Database)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, FromRow)]
struct NotificationRecord {
    id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    title: String,
    body: String,
    is_read: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct NotificationInput {
    recipient_email: String,
    title: String,
    body: String,
}

async fn create_notification(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<NotificationInput>,
) -> Result<(StatusCode, Json<NotificationRecord>), ApiError> {
    let sender_id = authenticated_user(&state, &headers)?;
    validate_notification_input(&input)?;
    let recipient_id = state
        .users
        .find_id_by_email(&input.recipient_email)
        .await
        .map_err(ApiError::Database)?
        .ok_or(ApiError::NotFound)?;
    let notification = sqlx::query_as::<_, NotificationRecord>(
        "INSERT INTO notifications (id, sender_id, recipient_id, title, body)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, sender_id, recipient_id, title, body, is_read, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(sender_id)
    .bind(recipient_id)
    .bind(input.title.trim())
    .bind(input.body.trim())
    .fetch_one(&state.database.0)
    .await
    .map_err(ApiError::Database)?;
    Ok((StatusCode::CREATED, Json(notification)))
}

async fn list_notifications(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<NotificationRecord>>, ApiError> {
    let user_id = authenticated_user(&state, &headers)?;
    let notifications = sqlx::query_as::<_, NotificationRecord>(
        "SELECT id, sender_id, recipient_id, title, body, is_read, created_at
         FROM notifications WHERE recipient_id = $1 ORDER BY created_at DESC LIMIT 100",
    )
    .bind(user_id)
    .fetch_all(&state.database.0)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(notifications))
}

async fn mark_notification_read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let user_id = authenticated_user(&state, &headers)?;
    let result =
        sqlx::query("UPDATE notifications SET is_read = TRUE WHERE id = $1 AND recipient_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&state.database.0)
            .await
            .map_err(ApiError::Database)?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
struct MailStatus {
    configured: bool,
    from: Option<String>,
}

async fn mail_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MailStatus>, ApiError> {
    authenticated_user(&state, &headers)?;
    Ok(Json(MailStatus {
        configured: state.mail.configured(),
        from: state.mail.from_address(),
    }))
}

#[derive(Debug, Deserialize)]
struct MailInput {
    to: String,
    subject: String,
    body: String,
}

async fn send_mail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<MailInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sender = authenticated_user(&state, &headers)?;
    validate_mail_input(&input)?;
    let to = input.to.trim();
    let subject = input.subject.trim();
    let reply_to = state
        .users
        .find_by_id(sender)
        .await
        .map_err(ApiError::Database)?
        .map(|user| (user.email, user.display_name));
    state
        .mail
        .send(to, subject, &input.body, reply_to)
        .await
        .map_err(ApiError::Mail)?;
    Ok(Json(serde_json::json!({ "status": "sent" })))
}

fn validate_credentials(input: &Credentials) -> Result<(), ApiError> {
    if !input.email.contains('@') || input.email.len() > 320 {
        return Err(ApiError::BadRequest("a valid email is required".into()));
    }
    if input.password.len() < 12 {
        return Err(ApiError::BadRequest(
            "password must contain at least 12 characters".into(),
        ));
    }
    if input.display_name.trim().is_empty() || input.display_name.len() > 120 {
        return Err(ApiError::BadRequest("display name is invalid".into()));
    }
    Ok(())
}

fn validate_notification_input(input: &NotificationInput) -> Result<(), ApiError> {
    if input.recipient_email.len() > 320 || !input.recipient_email.contains('@') {
        return Err(ApiError::BadRequest(
            "a valid recipient email is required".into(),
        ));
    }
    if input.title.trim().is_empty() || input.title.len() > 160 {
        return Err(ApiError::BadRequest("notification title is invalid".into()));
    }
    if input.body.trim().is_empty() || input.body.len() > 4000 {
        return Err(ApiError::BadRequest("notification body is invalid".into()));
    }
    Ok(())
}

fn validate_mail_input(input: &MailInput) -> Result<(), ApiError> {
    let to = input.to.trim();
    if !to.contains('@') || to.len() > 320 {
        return Err(ApiError::BadRequest(
            "a valid recipient email is required".into(),
        ));
    }
    let subject = input.subject.trim();
    if subject.is_empty() || subject.len() > 200 {
        return Err(ApiError::BadRequest("email subject is invalid".into()));
    }
    if input.body.trim().is_empty() || input.body.len() > 100_000 {
        return Err(ApiError::BadRequest("email body is invalid".into()));
    }
    Ok(())
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    value
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)
}

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<Uuid, ApiError> {
    state
        .auth
        .verify(bearer_token(headers)?)
        .map_err(ApiError::Auth)
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    Conflict(String),
    Unauthorized,
    NotFound,
    Auth(AuthError),
    Mail(MailError),
    Database(sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Conflict(message) => (StatusCode::CONFLICT, "conflict", message),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "authentication required".into(),
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "resource not found".into(),
            ),
            Self::Auth(error) => (StatusCode::UNAUTHORIZED, "auth_error", error.to_string()),
            Self::Mail(error) => {
                let message = error.to_string();
                let (status, code) = match error {
                    MailError::NotConfigured => {
                        (StatusCode::SERVICE_UNAVAILABLE, "mail_not_configured")
                    }
                    MailError::InvalidRecipient => {
                        (StatusCode::BAD_REQUEST, "invalid_recipient")
                    }
                    MailError::InvalidReplyTo => {
                        (StatusCode::BAD_REQUEST, "invalid_reply_to")
                    }
                    _ => (StatusCode::BAD_GATEWAY, "mail_delivery_failed"),
                };
                (status, code, message)
            }
            Self::Database(error) => {
                tracing::error!(%error, "database request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal server error".into(),
                )
            }
        };
        (
            status,
            Json(serde_json::json!({ "code": code, "message": message })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::AuthService, database::Database, mailer::MailService, sync::SyncService,
        users::UserService,
    };
    use axum::http::HeaderValue;
    use sqlx::postgres::PgPoolOptions;
    use std::{env, time::Duration};

    const TEST_SECRET: &str = "api-tests-jwt-secret-at-least-32-characters";
    // Shared with server/tests/database_integration.rs so parallel test binaries
    // cannot race on CREATE TABLE IF NOT EXISTS.
    const MIGRATION_LOCK: i64 = 0x454C5952415F5445;

    fn credentials(email: &str, password: &str, display_name: &str) -> Credentials {
        Credentials {
            email: email.to_string(),
            password: password.to_string(),
            display_name: display_name.to_string(),
        }
    }

    fn auth_header(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let value = format!("Bearer {token}");
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&value).expect("valid bearer header"),
        );
        headers
    }

    /// Resolve a test database without ever failing a plain `cargo test`.
    ///
    /// `ELYRA_TEST_DATABASE_URL` is an explicit opt-in: when set, connection and
    /// migration failures fail the test. A fallback `DATABASE_URL` is used only
    /// when it is reachable; otherwise the DB-backed cases skip themselves.
    fn test_database_url() -> Option<(String, bool)> {
        for (name, explicit) in [("ELYRA_TEST_DATABASE_URL", true), ("DATABASE_URL", false)] {
            if let Ok(url) = env::var(name) {
                if !url.trim().is_empty() {
                    return Some((url, explicit));
                }
            }
        }
        None
    }

    async fn migrate_once(database: &Database) -> Result<(), sqlx::Error> {
        let mut connection = database.0.acquire().await?;
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(MIGRATION_LOCK)
            .execute(&mut *connection)
            .await?;
        let migrated = database.migrate().await;
        let unlocked = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(MIGRATION_LOCK)
            .execute(&mut *connection)
            .await;
        migrated?;
        unlocked.map(|_| ())
    }

    async fn test_state() -> Option<Arc<AppState>> {
        let Some((url, explicit)) = test_database_url() else {
            eprintln!("skipping database-backed API test: set ELYRA_TEST_DATABASE_URL to enable it");
            return None;
        };
        let pool = match PgPoolOptions::new()
            .max_connections(3)
            .acquire_timeout(Duration::from_secs(2))
            .connect(&url)
            .await
        {
            Ok(pool) => pool,
            Err(error) if explicit => panic!("ELYRA_TEST_DATABASE_URL is unusable: {error}"),
            Err(error) => {
                eprintln!("skipping database-backed API test: {error}");
                return None;
            }
        };
        let database = Database(pool);
        if let Err(error) = migrate_once(&database).await {
            if explicit {
                panic!("test database migration failed: {error}");
            }
            eprintln!("skipping database-backed API test: migration failed: {error}");
            return None;
        }
        Some(Arc::new(AppState {
            database: database.clone(),
            auth: AuthService::new(TEST_SECRET, 24),
            users: UserService::new(database.clone()),
            sync: SyncService::new(database.clone()),
            mail: MailService::unconfigured(),
        }))
    }

    async fn cleanup_user(state: &AppState, id: Uuid) {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&state.database.0)
            .await
            .expect("delete test user");
    }

    #[test]
    fn credential_validation_covers_registration_and_login_input() {
        assert!(validate_credentials(&credentials(
            "user@example.test",
            "correct horse battery",
            "Test User"
        ))
        .is_ok());
        assert!(matches!(
            validate_credentials(&credentials("no-at-sign", "correct horse battery", "User")),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            validate_credentials(&credentials("user@example.test", "short", "User")),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            validate_credentials(&credentials("user@example.test", "correct horse battery", "  ")),
            Err(ApiError::BadRequest(_))
        ));
    }

    fn notification(to: &str, title: &str, body: &str) -> NotificationInput {
        NotificationInput {
            recipient_email: to.to_string(),
            title: title.to_string(),
            body: body.to_string(),
        }
    }

    fn mail(to: &str, subject: &str, body: &str) -> MailInput {
        MailInput {
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
        }
    }

    #[test]
    fn notification_and_mail_input_validation_is_strict() {
        assert!(validate_notification_input(&notification(
            "friend@example.test",
            "Hello",
            "Body"
        ))
        .is_ok());
        assert!(matches!(
            validate_notification_input(&notification("not-an-email", "Hello", "Body")),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            validate_notification_input(&notification("friend@example.test", " ", "Body")),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            validate_notification_input(&notification(
                "friend@example.test",
                "Hello",
                &"x".repeat(4001)
            )),
            Err(ApiError::BadRequest(_))
        ));

        assert!(validate_mail_input(&mail("friend@example.test", "Hello", "Body")).is_ok());
        assert!(matches!(
            validate_mail_input(&mail("", "Hello", "Body")),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            validate_mail_input(&mail("friend@example.test", " ", "Body")),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            validate_mail_input(&mail("friend@example.test", "Hello", "")),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    fn bearer_header_parsing_rejects_missing_and_wrong_schemes() {
        assert!(matches!(
            bearer_token(&HeaderMap::new()),
            Err(ApiError::Unauthorized)
        ));
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert!(matches!(
            bearer_token(&headers),
            Err(ApiError::Unauthorized)
        ));
        assert_eq!(
            bearer_token(&auth_header("token-value")).expect("bearer token"),
            "token-value"
        );
    }

    #[test]
    fn mail_errors_map_to_stable_http_statuses() {
        assert_eq!(
            ApiError::Mail(MailError::NotConfigured)
                .into_response()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            ApiError::Mail(MailError::InvalidRecipient)
                .into_response()
                .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::Mail(MailError::Send("timeout".into()))
                .into_response()
                .status(),
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            ApiError::Unauthorized.into_response().status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn registration_login_and_sync_workflows() {
        let Some(state) = test_state().await else {
            return;
        };
        let email = format!("{}@example.test", Uuid::new_v4());
        let password = "correct horse battery staple";

        let session = register(
            State(state.clone()),
            Json(credentials(&email, password, "Test User")),
        )
        .await
        .expect("register")
        .0;
        assert_eq!(session.user.email, email);
        assert!(!session.access_token.is_empty());
        assert!(!session.expires_at.is_empty());

        let signed_in = login(
            State(state.clone()),
            Json(credentials(&email, password, "Ignored")),
        )
        .await
        .expect("login")
        .0;
        assert_eq!(signed_in.user.id, session.user.id);

        let me_response = me(State(state.clone()), auth_header(&session.access_token))
            .await
            .expect("me")
            .0;
        assert_eq!(me_response.id, session.user.id);

        let before = sync_cursor(State(state.clone()), auth_header(&session.access_token))
            .await
            .expect("empty cursor")
            .0;
        assert!(before.cursor.is_none());
        assert!(before.synced_at.is_none());

        update_cursor(
            State(state.clone()),
            auth_header(&session.access_token),
            Json(CursorInput {
                cursor: Some("cursor-1".into()),
            }),
        )
        .await
        .expect("update cursor");
        let after = sync_cursor(State(state.clone()), auth_header(&session.access_token))
            .await
            .expect("updated cursor")
            .0;
        assert_eq!(after.cursor.as_deref(), Some("cursor-1"));
        assert!(after.synced_at.is_some());

        let status = mail_status(State(state.clone()), auth_header(&session.access_token))
            .await
            .expect("mail status")
            .0;
        assert!(!status.configured);
        assert_eq!(status.from, None);
        let error = match send_mail(
            State(state.clone()),
            auth_header(&session.access_token),
            Json(MailInput {
                to: "friend@example.test".into(),
                subject: "Hello".into(),
                body: "Body".into(),
            }),
        )
        .await
        {
            Ok(_) => panic!("disabled relay must fail closed"),
            Err(error) => error,
        };
        assert!(matches!(error, ApiError::Mail(MailError::NotConfigured)));

        cleanup_user(&state, session.user.id).await;
    }

    #[tokio::test]
    async fn duplicate_registration_and_wrong_password_are_rejected() {
        let Some(state) = test_state().await else {
            return;
        };
        let email = format!("{}@example.test", Uuid::new_v4());
        let session = register(
            State(state.clone()),
            Json(credentials(&email, "correct horse battery", "Test User")),
        )
        .await
        .expect("register")
        .0;

        assert!(matches!(
            register(
                State(state.clone()),
                Json(credentials(&email, "another long password", "Other"))
            )
            .await,
            Err(ApiError::Conflict(_))
        ));
        assert!(matches!(
            login(
                State(state.clone()),
                Json(credentials(&email, "wrong but long enough", "Ignored"))
            )
            .await,
            Err(ApiError::Auth(AuthError::InvalidCredentials))
        ));

        cleanup_user(&state, session.user.id).await;
    }

    #[tokio::test]
    async fn protected_routes_require_a_valid_bearer_token() {
        let Some(state) = test_state().await else {
            return;
        };
        assert!(matches!(
            me(State(state.clone()), HeaderMap::new()).await,
            Err(ApiError::Unauthorized)
        ));
        assert!(matches!(
            me(State(state.clone()), auth_header("not-a-jwt")).await,
            Err(ApiError::Auth(AuthError::InvalidToken))
        ));
    }

    #[tokio::test]
    async fn notifications_are_private_to_the_recipient() {
        let Some(state) = test_state().await else {
            return;
        };
        let sender_email = format!("{}@example.test", Uuid::new_v4());
        let recipient_email = format!("{}@example.test", Uuid::new_v4());
        let sender = register(
            State(state.clone()),
            Json(credentials(&sender_email, "correct horse battery", "Sender")),
        )
        .await
        .expect("register sender")
        .0;
        let recipient = register(
            State(state.clone()),
            Json(credentials(&recipient_email, "correct horse battery", "Recipient")),
        )
        .await
        .expect("register recipient")
        .0;

        let (status, created) = create_notification(
            State(state.clone()),
            auth_header(&sender.access_token),
            Json(NotificationInput {
                recipient_email: recipient_email.clone(),
                title: "Hello".into(),
                body: "Private message".into(),
            }),
        )
        .await
        .expect("create notification");
        assert_eq!(status, StatusCode::CREATED);
        let created = created.0;
        assert_eq!(created.sender_id, sender.user.id);
        assert_eq!(created.recipient_id, recipient.user.id);
        assert!(!created.is_read);

        let sender_list = list_notifications(State(state.clone()), auth_header(&sender.access_token))
            .await
            .expect("sender notifications")
            .0;
        assert!(sender_list.is_empty());
        let recipient_list =
            list_notifications(State(state.clone()), auth_header(&recipient.access_token))
                .await
                .expect("recipient notifications")
                .0;
        assert_eq!(recipient_list.len(), 1);

        assert!(matches!(
            mark_notification_read(
                State(state.clone()),
                auth_header(&sender.access_token),
                Path(created.id)
            )
            .await,
            Err(ApiError::NotFound)
        ));
        assert_eq!(
            mark_notification_read(
                State(state.clone()),
                auth_header(&recipient.access_token),
                Path(created.id)
            )
            .await
            .expect("mark notification read"),
            StatusCode::NO_CONTENT
        );
        let updated = list_notifications(State(state.clone()), auth_header(&recipient.access_token))
            .await
            .expect("updated notifications")
            .0;
        assert!(updated[0].is_read);

        cleanup_user(&state, sender.user.id).await;
        cleanup_user(&state, recipient.user.id).await;
    }
}
