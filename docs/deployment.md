# Deployment

Elyra's production deployment is a small Docker Compose stack for one Linux VPS.
It serves the public website, proxies the authenticated Rust API, stores server
data in PostgreSQL, and can terminate HTTPS with Caddy.

The complete step-by-step walkthrough — DNS, firewall, secrets, TLS, SMTP relay,
backups, and GitHub Actions — is in the
[server runbook](deploy/SERVER.md). This page is the overview and quick start.

## What gets deployed

```text
Caddy (optional, profile: tls)
  │  80/443, automatic Let's Encrypt certificates
  ▼
web (Nginx)
  │  static site + /api/ proxy + /health proxy
  ├──────────────▶ api (Rust/Axum, :8787)
  │                   │
  │                   ▼
  │                postgres (:5432)
  │
  └──────────────▶ redis (:6379, reserved for future use)
```

| Service | Image/build | Published | Purpose |
| --- | --- | --- | --- |
| `web` | `web/Dockerfile` (Nginx 1.27 Alpine) | `WEB_PORT` (default 8080) | Website and API proxy |
| `api` | `server/Dockerfile` (Rust + Debian slim) | No | Auth, users, sync, notifications, relay |
| `postgres` | `postgres:16-alpine` | No | Durable server data |
| `redis` | `redis:7-alpine` | No | Reserved; not used by the server yet |
| `caddy` | `caddy:2-alpine`, `tls` profile | 80/443 | TLS termination and HSTS |

Compose file: `infrastructure/deployment/docker-compose.prod.yml`.
Environment template: `infrastructure/deployment/.env.prod.example`.
Deploy script: `infrastructure/deployment/deploy.sh`.
Caddy config: `infrastructure/deployment/Caddyfile`.

## Requirements

- Linux server with Docker Engine 24+ and the Compose plugin.
- A domain or subdomain for HTTPS.
- SSH access with a non-root sudo user.
- 1–2 vCPU and 1–2 GB RAM is enough for a small deployment.
- Ports 80 and 443 open if you use Caddy.

Do not publish PostgreSQL (5432), Redis (6379), or the API (8787).

## Quick start without HTTPS

Use this only for a first test or behind an existing reverse proxy.

```sh
git clone https://github.com/imtiredwhenitalk/Elyra.git /opt/elyra
cd /opt/elyra
cp infrastructure/deployment/.env.prod.example infrastructure/deployment/.env.prod
chmod 600 infrastructure/deployment/.env.prod
```

Edit `.env.prod`:

```env
POSTGRES_PASSWORD=<long-random-value>
REDIS_PASSWORD=<different-long-random-value>
ELYRA_JWT_SECRET=<at-least-32-random-characters>
WEB_BIND=0.0.0.0
WEB_PORT=8080
```

Generate secrets with `openssl rand -hex 32`. Then:

```sh
./infrastructure/deployment/deploy.sh
```

The script validates the Compose configuration, builds the API and website
images, starts the stack, waits for `/health`, and prints the URLs. Open
`http://SERVER_IP:8080`.

## HTTPS with Caddy

1. Point a DNS `A` (and optional `AAAA`) record at the server.
2. Set in `.env.prod`:

```env
ELYRA_ENABLE_TLS=1
ELYRA_DOMAIN=mail.example.com
WEB_BIND=127.0.0.1
```

3. Deploy:

```sh
./infrastructure/deployment/deploy.sh --tls
```

Caddy obtains and renews the certificate. Public endpoints:

- Website: `https://mail.example.com`
- API base: `https://mail.example.com/api`
- Health: `https://mail.example.com/api/health`

`WEB_BIND=127.0.0.1` keeps the plain HTTP website port off the public interface.
Only Caddy's 80/443 are reachable.

## Point the desktop app at the server

In Elyra: **Settings → Elyra Cloud → Server URL**, enter
`https://mail.example.com/api`, save, then connect and register.

For release installers, set the GitHub repository variable
`ELYRA_DEFAULT_API_URL=https://mail.example.com/api` before tagging. The release
workflow bakes it into the build.

The API base must:

- Use HTTPS in production.
- Include `/api`.
- Have no trailing slash.

## Optional mail relay

The relay lets registered users send to external addresses without configuring
their own SMTP account. It is disabled unless `ELYRA_SMTP_HOST` is set.

```env
ELYRA_SMTP_HOST=smtp.example.com
ELYRA_SMTP_PORT=587
ELYRA_SMTP_SECURITY=starttls
ELYRA_SMTP_USERNAME=no-reply@example.com
ELYRA_SMTP_PASSWORD=app-password
ELYRA_SMTP_FROM=no-reply@example.com
ELYRA_SMTP_FROM_NAME=Elyra
```

- Use `tls` on port 465 for implicit TLS; `starttls` on port 587 otherwise.
- Publish SPF, DKIM, and DMARC for the sending domain.
- Use a dedicated mailbox or transactional provider.
- Any registered user can send. Add rate limiting before public exposure.

See [Server and API → SMTP relay](server.md#smtp-relay) for behavior and error
codes.

## Deploy script options

```sh
./infrastructure/deployment/deploy.sh --check     # validate configuration only
./infrastructure/deployment/deploy.sh --backup    # dump PostgreSQL first
./infrastructure/deployment/deploy.sh --no-build  # reuse existing images
./infrastructure/deployment/deploy.sh --tls       # enable Caddy profile
./infrastructure/deployment/deploy.sh --help
```

The script exits with a clear error if Docker, the Compose plugin, or the env
file is missing. It never stores secrets outside `.env.prod`.

## GitHub Actions auto-deploy

The `Deploy server` workflow is opt-in.

1. Create an SSH deploy key and add its public half to
   `~/.ssh/authorized_keys` on the server.
2. Add repository secrets:
   - `ELYRA_DEPLOY_HOST`
   - `ELYRA_DEPLOY_USER`
   - `ELYRA_DEPLOY_KEY`
   - `ELYRA_DEPLOY_PORT` (optional, default 22)
   - `ELYRA_DEPLOY_PATH` (for example `/opt/elyra`)
3. Add repository variable `ELYRA_DEPLOY_ENABLED=true`.
4. Optionally set `ELYRA_DEFAULT_API_URL`.

The workflow triggers on pushes to `main` that touch `server/`, `web/`,
`packages/`, `infrastructure/deployment/`, `infrastructure/web/`, `Cargo.toml`,
`Cargo.lock`, or the workflow itself. It can also be started manually.

It runs:

```sh
git fetch --prune origin main
git reset --hard origin/main
./infrastructure/deployment/deploy.sh --backup
```

The previous containers keep running until the new build succeeds.

## Backups

Manual backup:

```sh
./infrastructure/deployment/deploy.sh --backup
```

Or directly:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml \
  exec -T postgres sh -c 'pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB"' \
  | gzip > backups/elyra-$(date +%Y%m%d-%H%M%S).sql.gz
```

Restore:

```sh
gunzip -c backups/elyra-YYYYMMDD-HHMMSS.sql.gz \
  | docker compose --env-file infrastructure/deployment/.env.prod \
      -f infrastructure/deployment/docker-compose.prod.yml \
      exec -T postgres sh -c 'psql -U "$POSTGRES_USER" "$POSTGRES_DB"'
```

Copy backups off the server and test restores periodically. The server stores no
separate file uploads.

## Update

```sh
cd /opt/elyra
git pull --ff-only
./infrastructure/deployment/deploy.sh --backup
```

Schema changes run at API startup as idempotent `CREATE TABLE IF NOT EXISTS`
statements. Review `server/src/database.rs` before upgrading across major
versions; destructive migrations are not automated.

## Rollback

The stack is built from the checked-out source. To roll back:

```sh
cd /opt/elyra
git checkout <previous-good-tag>
./infrastructure/deployment/deploy.sh --backup
```

If the database schema changed in a non-backward-compatible way, restore the
matching PostgreSQL backup. Keep the previous `.env.prod` values available; a
JWT secret change invalidates all sessions.

## Hardening checklist

- [ ] Firewall allows only SSH, HTTP, and HTTPS.
- [ ] `POSTGRES_PASSWORD`, `REDIS_PASSWORD`, and `ELYRA_JWT_SECRET` are unique.
- [ ] `.env.prod` is mode `600` and never committed.
- [ ] PostgreSQL, Redis, and the API are not published.
- [ ] `WEB_BIND=127.0.0.1` when Caddy is enabled.
- [ ] DNS points at the server before requesting a certificate.
- [ ] Backups run on a schedule and restores are tested.
- [ ] SPF, DKIM, and DMARC exist for the relay domain.
- [ ] Registration and relay abuse controls are planned before public sign-ups.
- [ ] Logs and `/health` are monitored.

## Troubleshooting

The full symptom table is in the [server runbook](deploy/SERVER.md#10-troubleshooting)
and [Troubleshooting](troubleshooting.md). Quick checks:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml ps
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml logs --tail=100 api
curl -fsS https://mail.example.com/api/health
```

## Related pages

- [Server runbook](deploy/SERVER.md) — complete VPS walkthrough.
- [Server and API](server.md) — routes, data model, relay.
- [Security](security.md) — threat model and operator checklist.
- [Configuration reference](reference/configuration.md) — deployment variables.
- [Troubleshooting](troubleshooting.md) — common failures.
