# Elyra server

The server is a small authenticated REST foundation for account metadata and
sync cursors. It never receives desktop encryption keys or raw local message
secrets.

## Run locally

1. Start PostgreSQL with `infrastructure/docker/docker-compose.yaml`.
2. Copy `server/.env.example` to `.env` and set a random
   `ELYRA_JWT_SECRET` with at least 32 characters.
3. Run `cargo run -p elyra-server`.

Endpoints:

- `GET /health`
- `POST /v1/auth/register`
- `POST /v1/auth/login`
- `GET /v1/users/me`
- `GET /v1/sync/cursor`
- `PUT /v1/sync/cursor`
- `GET /v1/notifications`
- `POST /v1/notifications`
- `PATCH /v1/notifications/:id/read`
- `GET /v1/mail/status`
- `POST /v1/mail/send`

Passwords are hashed with Argon2id. Access tokens are signed JWTs and should be
sent as `Authorization: Bearer <token>`.

To send an in-app notification, the authenticated client posts
`recipient_email`, `title`, and `body`. The recipient must already have an
Elyra account. Notifications are scoped to the recipient and cannot be read or
marked as read by another user.

## Outbound mail relay

`POST /v1/mail/send` accepts `to`, `subject`, and `body` from an authenticated
user and forwards the message through the SMTP relay configured with
`ELYRA_SMTP_HOST`, `ELYRA_SMTP_PORT`, `ELYRA_SMTP_SECURITY` (`starttls` or
`tls`), `ELYRA_SMTP_USERNAME`, `ELYRA_SMTP_PASSWORD`, `ELYRA_SMTP_FROM`, and
`ELYRA_SMTP_FROM_NAME`. The desktop app uses it automatically when the user has
no local SMTP account, so they can write to any external address without
configuring a mail provider.

The relay is disabled unless `ELYRA_SMTP_HOST` is set. `GET /v1/mail/status`
reports whether it is configured. Messages are sent from the relay address with
`Reply-To` set to the signed-in user's Elyra email, so replies return to them.
Use a dedicated sending mailbox or a transactional provider and publish SPF,
DKIM, and DMARC records. The server never stores desktop IMAP credentials.

## Container release

The production image is built from `server/Dockerfile`. The complete
website/API/database stack is started with:

```sh
docker compose -f infrastructure/deployment/docker-compose.prod.yml up -d --build
```

The website is exposed on `WEB_PORT` (default `8080`) and proxies `/api` to
the private API container. Set `POSTGRES_PASSWORD`, `REDIS_PASSWORD`, and
`ELYRA_JWT_SECRET` in the deployment environment; no production secret has a
repository default.
