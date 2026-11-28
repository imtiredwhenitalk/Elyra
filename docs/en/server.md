> 🌐 **English** · [Українська](../ua/server.md)


# Server and API

The Elyra server is a small authenticated Rust service. It provides account
registration and login, user metadata, sync cursors, in-app notifications, and an
optional outbound SMTP relay. It does **not** store mailboxes, IMAP passwords,
desktop encryption keys, or message bodies.

This page is the technical reference. For a production VPS walkthrough, use
[Deployment](deployment.md) and the [server runbook](deploy/SERVER.md).

## What the server owns

| Stored | Not stored |
| --- | --- |
| User id, email, display name, Argon2id password hash | IMAP/SMTP passwords |
| Sync cursor per user | Mailbox contents or attachments |
| In-app notifications | Desktop encryption keys |
| Optional relay configuration (environment only) | Relay message bodies |

The desktop talks to the server over HTTPS. IMAP and SMTP traffic goes directly
from the desktop to the mail provider and never passes through the Elyra server.

## Local development

### 1. Start PostgreSQL

Use the development Compose stack:

```sh
./infrastructure/build/program.sh up
```

Or start only PostgreSQL from `infrastructure/docker/docker-compose.yaml`.
The development stack also provides Redis and Mailpit.

### 2. Configure the environment

```sh
cp server/.env.example server/.env
```

Set at least:

```env
DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra
ELYRA_JWT_SECRET=replace-with-at-least-32-random-characters
ELYRA_SERVER_ADDR=127.0.0.1:8787
```

Generate a secret with `openssl rand -hex 32`.

### 3. Run

```sh
cargo run -p elyra-server
```

The server loads `server/.env` via `dotenvy`, runs `CREATE TABLE IF NOT EXISTS`
migrations, binds the configured address, and logs
`Elyra server listening`.

### 4. Smoke test

```sh
curl -fsS http://127.0.0.1:8787/health
# {"service":"elyra-server","status":"ok"}

curl -fsS -X POST http://127.0.0.1:8787/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"you@example.com","password":"correct horse battery","display_name":"You"}'
```

Use the returned `access_token` as `Authorization: Bearer <token>` on protected
routes.

## Configuration

All server variables are listed in the
[configuration reference](reference/configuration.md#server). The required ones:

| Variable | Required | Notes |
| --- | --- | --- |
| `DATABASE_URL` | Yes | PostgreSQL connection string; pool max 10 |
| `ELYRA_JWT_SECRET` | Yes | At least 32 characters; server refuses to start otherwise |
| `ELYRA_SERVER_ADDR` | No | Default `127.0.0.1:8787`; Docker sets `0.0.0.0:8787` |
| `ELYRA_ACCESS_TOKEN_TTL_HOURS` | No | Default `24` |
| `RUST_LOG` | No | Default `elyra_server=info,tower_http=info` |
| `ELYRA_SMTP_HOST` | No | Empty disables the relay |
| `ELYRA_SMTP_FROM` | When relay is on | Invalid or missing value fails startup |
| Other `ELYRA_SMTP_*` | No | Port, security, username, password, from name |

The server has no production defaults for secrets. The production Compose file
uses `${VAR:?message}` so missing values fail fast.

## Startup and runtime

```text
dotenvy::dotenv
  → tracing subscriber
  → Database::connect_from_env
  → Database::migrate
  → AuthService::from_env
  → MailService::from_env
  → axum::serve(router)
  → graceful shutdown on Ctrl+C
```

Middleware:

- `CorsLayer::permissive()` — allows browser preview and local tools to call the
  API. In production, the website proxies same-origin requests; permissive CORS
  is still enabled by the server.
- `TraceLayer::new_for_http()` — request tracing through `tower-http`.

The server is stateless apart from PostgreSQL and environment configuration.
JWTs remain valid across restarts as long as `ELYRA_JWT_SECRET` does not change.

## Data model

```sql
users (
  id UUID PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)

sync_cursors (
  user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  cursor TEXT,
  synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)

notifications (
  id UUID PRIMARY KEY,
  sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  is_read BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
```

Migrations are idempotent `CREATE TABLE IF NOT EXISTS` statements executed at
startup. There is no migration version table. Destructive schema changes are not
automated; review `server/src/database.rs` before upgrading across major
versions.

## Authentication

### Registration and login

`POST /v1/auth/register` and `POST /v1/auth/login` accept:

```json
{
  "email": "you@example.com",
  "password": "correct horse battery",
  "display_name": "You"
}
```

`display_name` is optional on login and defaults to `Elyra user` when omitted
from either request.

Validation:

- Email must contain `@` and be at most 320 characters.
- Password must be at least 12 characters.
- Display name must be non-empty after trimming and at most 120 characters.
- Registration returns `409 conflict` if the exact email is already registered.

Response:

```json
{
  "access_token": "<jwt>",
  "expires_at": "2026-11-17T12:00:00Z",
  "user": {
    "id": "0f3f6c1e-...",
    "email": "you@example.com",
    "display_name": "You",
    "created_at": "2026-11-16T12:00:00Z"
  }
}
```

### Tokens

- JWT HS256 with claims `{ "sub": "<user uuid>", "exp": <unix seconds> }`.
- TTL from `ELYRA_ACCESS_TOKEN_TTL_HOURS`, default 24 hours.
- Send as `Authorization: Bearer <token>`.
- There is no refresh token. When a token expires, sign in again.
- Changing `ELYRA_JWT_SECRET` invalidates every existing token.

### Password hashing

Passwords are hashed with Argon2id using `argon2::Argon2::default()` and a random
salt from the OS. The hash is stored in `users.password_hash`; plaintext
passwords are never stored or logged.

## API reference

Base URL in local development: `http://127.0.0.1:8787`.
Base URL in production: `https://<domain>/api` (the website Nginx proxy strips
the `/api` prefix before forwarding to the API container).

All request and response bodies are JSON. Protected routes require the bearer
header.

### `GET /health`

Public liveness endpoint.

```json
{ "status": "ok", "service": "elyra-server" }
```

The production Compose health check greps for `"status":"ok"` inside the
container.

### `POST /v1/auth/register`

Public. Creates a user and returns a session.

Request:

```json
{
  "email": "you@example.com",
  "password": "correct horse battery",
  "display_name": "You"
}
```

Response `200`:

```json
{
  "access_token": "<jwt>",
  "expires_at": "2026-11-17T12:00:00Z",
  "user": {
    "id": "0f3f6c1e-...",
    "email": "you@example.com",
    "display_name": "You",
    "created_at": "2026-11-16T12:00:00Z"
  }
}
```

Errors: `400 bad_request`, `409 conflict`, `500 internal_error`.

### `POST /v1/auth/login`

Public. Verifies credentials and returns the same session shape as registration.

Errors: `400 bad_request`, `401 auth_error` with
`invalid credentials`, `500 internal_error`.

### `GET /v1/users/me`

Protected. Returns the current `UserRecord`.

```json
{
  "id": "0f3f6c1e-...",
  "email": "you@example.com",
  "display_name": "You",
  "created_at": "2026-11-16T12:00:00Z"
}
```

Errors: `401 unauthorized`, `401 auth_error`, `404 not_found`,
`500 internal_error`.

### `GET /v1/sync/cursor`

Protected. Returns the caller's cursor.

```json
{ "cursor": "opaque-value-or-null", "synced_at": "2026-11-16T12:00:00Z" }
```

If no cursor exists:

```json
{ "cursor": null, "synced_at": null }
```

The cursor is an opaque string. The 0.4.0 desktop exposes get/update commands but
does not run a scheduled sync loop.

### `PUT /v1/sync/cursor`

Protected. Replaces the caller's cursor and updates `synced_at`.

Request:

```json
{ "cursor": "opaque-value" }
```

`cursor` may be `null` to clear it. Response: `204 No Content`.

Errors: `401 unauthorized`, `500 internal_error`.

### `GET /v1/notifications`

Protected. Returns up to 100 notifications for the caller, newest first.

```json
[
  {
    "id": "7a1c...",
    "sender_id": "0f3f...",
    "recipient_id": "9b2d...",
    "title": "Welcome",
    "body": "Your Elyra account is ready.",
    "is_read": false,
    "created_at": "2026-11-16T12:00:00Z"
  }
]
```

Notifications are scoped to `recipient_id = caller`. Other users' notifications
are never returned.

### `POST /v1/notifications`

Protected. Sends an in-app notification to another registered user.

Request:

```json
{
  "recipient_email": "friend@example.com",
  "title": "Hello",
  "body": "This is an in-app notification, not email."
}
```

Validation:

| Field | Rule |
| --- | --- |
| `recipient_email` | Contains `@`, at most 320 characters |
| `title` | Non-empty after trim, at most 160 characters |
| `body` | Non-empty after trim, at most 4000 characters |

Response `201`:

```json
{
  "id": "7a1c...",
  "sender_id": "0f3f...",
  "recipient_id": "9b2d...",
  "title": "Hello",
  "body": "This is an in-app notification, not email.",
  "is_read": false,
  "created_at": "2026-11-16T12:00:00Z"
}
```

Errors: `400 bad_request`, `401 unauthorized`, `404 not_found` when the
recipient does not exist, `500 internal_error`.

### `PATCH /v1/notifications/:id/read`

Protected. Marks one notification read **only if** it belongs to the caller.

Response: `204 No Content`.

Errors: `401 unauthorized`, `404 not_found` if the id does not exist or belongs
to another user.

### `GET /v1/mail/status`

Protected. Reports whether the relay is configured.

```json
{ "configured": true, "from": "no-reply@example.com" }
```

When the relay is disabled:

```json
{ "configured": false, "from": null }
```

### `POST /v1/mail/send`

Protected. Sends a plain-text message through the server relay.

Request:

```json
{
  "to": "friend@example.com",
  "subject": "Hello from Elyra",
  "body": "Plain text body."
}
```

Validation:

| Field | Rule |
| --- | --- |
| `to` | Contains `@`, at most 320 characters |
| `subject` | Non-empty after trim, at most 200 characters |
| `body` | Non-empty after trim, at most 100 000 characters |

Response `200`:

```json
{ "status": "sent" }
```

The message is sent from `ELYRA_SMTP_FROM` (with `ELYRA_SMTP_FROM_NAME`) and
`Reply-To` is set to the signed-in user's email and display name. The server
does not persist the body.

Errors:

| Status | Code | Meaning |
| --- | --- | --- |
| 400 | `bad_request` | Invalid recipient, subject, or body |
| 400 | `invalid_recipient` | `lettre` rejected the recipient address |
| 400 | `invalid_reply_to` | The signed-in user's email cannot be used as Reply-To |
| 401 | `unauthorized` / `auth_error` | Missing or invalid token |
| 503 | `mail_not_configured` | `ELYRA_SMTP_HOST` is empty |
| 502 | `mail_delivery_failed` | SMTP connection, authentication, or delivery failed |

## Error format

Every API error uses the same shape:

```json
{ "code": "bad_request", "message": "password must contain at least 12 characters" }
```

| Status | Code | When |
| --- | --- | --- |
| 400 | `bad_request` | Validation failure |
| 401 | `unauthorized` | Missing or malformed bearer header |
| 401 | `auth_error` | Invalid credentials or invalid/expired token |
| 404 | `not_found` | Missing user, notification, or resource |
| 409 | `conflict` | Email already registered |
| 500 | `internal_error` | Database or unexpected failure; details are logged, not returned |
| 502 | `mail_delivery_failed` | Relay could not deliver |
| 503 | `mail_not_configured` | Relay is disabled |

Database errors are logged with `tracing::error!` and returned as a generic
`internal_error` message so internal details do not leak to clients.

## SMTP relay

The relay is optional and disabled unless `ELYRA_SMTP_HOST` is set.

```env
ELYRA_SMTP_HOST=smtp.example.com
ELYRA_SMTP_PORT=587
ELYRA_SMTP_SECURITY=starttls
ELYRA_SMTP_USERNAME=no-reply@example.com
ELYRA_SMTP_PASSWORD=app-password
ELYRA_SMTP_FROM=no-reply@example.com
ELYRA_SMTP_FROM_NAME=Elyra
```

Behavior:

- `ELYRA_SMTP_SECURITY=tls` uses implicit TLS (typically port 465).
- Any other value, including the default, uses STARTTLS (typically port 587).
- If the host is set but `ELYRA_SMTP_FROM` is missing or invalid, the server
  **fails to start** instead of running with a broken relay.
- The relay uses `lettre` with rustls and certificate validation.
- The body is not persisted.
- Any registered user can use the relay. There is no rate limit, quota, or
  per-user abuse control in 0.4.0.

Before exposing the relay publicly, publish SPF, DKIM, and DMARC records for the
sending domain and add rate limiting. Use a dedicated sending mailbox or a
transactional provider, not a personal inbox.

## Desktop integration

The desktop client calls the server through `cloud.rs` and
`services/sendemail.rs`.

| Desktop command | Server route |
| --- | --- |
| `cloud_register` | `POST /v1/auth/register` |
| `cloud_login` | `POST /v1/auth/login` |
| `cloud_session` | Local keychain only |
| `cloud_logout` | Local keychain + `settings` cleanup |
| `cloud_list_notifications` | `GET /v1/notifications` |
| `cloud_send_notification` | `POST /v1/notifications` |
| `cloud_mark_notification_read` | `PATCH /v1/notifications/:id/read` |
| `cloud_sync_cursor` | `GET /v1/sync/cursor` |
| `cloud_update_cursor` | `PUT /v1/sync/cursor` |
| `cloud_health` | `GET /health` |
| `cloud_mail_status` | `GET /v1/mail/status` |
| `send_email` fallback | `POST /v1/mail/send` |

### API URL resolution

The desktop resolves the API base in this order:

1. `ELYRA_API_URL` environment variable.
2. SQLite `settings.api_url`.
3. `ELYRA_DEFAULT_API_URL` compiled into the installer.
4. `http://127.0.0.1:8787`.

Trailing slashes are removed. Production URLs must include `/api` because the
public Nginx site proxies `/api/` to the API container.

## Testing

The server has pure tests that run anywhere and database-backed tests that skip
gracefully unless a disposable PostgreSQL instance is provided:

```sh
cargo test -p elyra-server

ELYRA_TEST_DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra_test \
  cargo test -p elyra-server
```

`ELYRA_TEST_DATABASE_URL` is an explicit opt-in; an unusable URL fails the run.
`DATABASE_URL` is only a fallback when reachable. The tests cover auth and JWT
handling, API validation, bearer-header and error handling, register/login, sync
cursors, notification privacy, disabled SMTP relay behavior, and schema
constraints. No test sends real SMTP traffic. See
[Development → Automated tests](development.md#automated-tests) for details and
[Production verification](deploy/VERIFY.md) for the deployment smoke tests.

## Operational notes

### Health and logs

```sh
curl -fsS http://127.0.0.1:8787/health
RUST_LOG=elyra_server=debug,tower_http=debug cargo run -p elyra-server
```

In production:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml logs -f api
```

### Database access

The server connects with a pool of up to 10 connections. PostgreSQL is not
published in the production stack. Use `docker compose exec` for maintenance.

### Backups

Back up PostgreSQL with `pg_dump`; see the [server runbook](deploy/SERVER.md#9-backups-and-maintenance).
The server stores no files that need a separate backup.

### CORS

The API enables permissive CORS. The production website proxies same-origin
requests, so CORS is not required for the website. It exists for browser preview
and local tooling. If you expose the API directly, restrict CORS to trusted
origins.

### Known server limitations

- Email addresses are trimmed and lowercased at the API boundary and a
  `lower(email)` unique index prevents case variants. Databases upgraded with
  duplicate case variants must be merged before the index can be created; the
  server reports the conflicting address instead of starting with a broken
  schema.
- No refresh tokens, token revocation, password reset, email verification, or
  multi-factor authentication.
- No admin role, invite code, registration rate limit, or relay quota.
- No WebSocket or realtime channel.
- Redis is deployed but unused by the server.
- Only plain-text outbound bodies are accepted; attachments are not supported by
  the relay endpoint.

These are tracked in [`task.md`](../../task.md).

## Related pages

- [Architecture](architecture.md) — how the server fits into the system.
- [Security](security.md) — trust boundaries and threat model.
- [Configuration reference](reference/configuration.md) — every variable.
- [Deployment](deployment.md) — production stack and HTTPS.
- [Troubleshooting](troubleshooting.md) — server symptoms and fixes.
