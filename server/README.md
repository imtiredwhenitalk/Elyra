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

Passwords are hashed with Argon2id. Access tokens are signed JWTs and should be
sent as `Authorization: Bearer <token>`.

To send an in-app notification, the authenticated client posts
`recipient_email`, `title`, and `body`. The recipient must already have an
Elyra account. Notifications are scoped to the recipient and cannot be read or
marked as read by another user.

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
