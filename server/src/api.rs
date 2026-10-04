use axum::{
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::{auth::AuthError, users::UserRecord, AppState};

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

fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<Uuid, ApiError> {
    let value = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    let token = value
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;
    state.auth.verify(token).map_err(ApiError::Auth)
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    Conflict(String),
    Unauthorized,
    NotFound,
    Auth(AuthError),
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
