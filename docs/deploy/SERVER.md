# Elyra server deployment

This guide takes a fresh Linux server to a working Elyra deployment: website,
authenticated Rust API, PostgreSQL, optional HTTPS, and optional outbound mail
relay. It is designed for a single small VPS (1–2 vCPU, 1–2 GB RAM).

The desktop app keeps mail and IMAP credentials locally. The server only stores
Elyra accounts, sync cursors, notifications, and the optional SMTP relay
configuration.

## 1. Prerequisites

- Ubuntu 22.04/24.04 (or any Linux with Docker Engine 24+).
- A domain or subdomain, for example `mail.example.com`.
- SSH access with a non-root sudo user.
- Docker Engine and the Compose plugin:

```sh
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker "$USER"
# Log out and back in so the docker group applies.
docker compose version
```

Open only SSH, HTTP, and HTTPS in the firewall:

```sh
sudo ufw allow OpenSSH
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable
```

Do **not** publish PostgreSQL (`5432`), Redis (`6379`), or the API (`8787`).
The production Compose file keeps them on a private Docker network.

## 2. Get the code

```sh
sudo mkdir -p /opt/elyra
sudo chown "$USER":"$USER" /opt/elyra
git clone https://github.com/imtiredwhenitalk/Elyra.git /opt/elyra
cd /opt/elyra
```

## 3. Configure secrets

```sh
cp infrastructure/deployment/.env.prod.example infrastructure/deployment/.env.prod
chmod 600 infrastructure/deployment/.env.prod
```

Edit `infrastructure/deployment/.env.prod` and replace every placeholder.
Generate secrets with:

```sh
openssl rand -hex 32
```

Required values:

| Variable | Purpose |
| --- | --- |
| `POSTGRES_PASSWORD` | PostgreSQL password |
| `REDIS_PASSWORD` | Redis password |
| `ELYRA_JWT_SECRET` | JWT signing key, at least 32 characters |
| `WEB_BIND` | `127.0.0.1` with Caddy, `0.0.0.0` without |
| `WEB_PORT` | Host port for the website, default `8080` |

`.env.prod` is ignored by Git. Never commit it.

## 4. Deploy

Without HTTPS (quick test, open `http://SERVER_IP:8080`):

```sh
./infrastructure/deployment/deploy.sh
```

The script validates the Compose file, builds the API and website images,
starts the stack, waits for `/health`, and prints the URLs. Use `--backup` to
dump PostgreSQL first and `--check` to only validate the configuration.

Check the stack:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml ps
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml logs -f api
```

## 5. HTTPS with Caddy

Caddy obtains and renews Let's Encrypt certificates automatically.

1. Create a DNS `A` record for `ELYRA_DOMAIN` pointing at the server. If you
   use IPv6, add an `AAAA` record too.
2. Set these values in `.env.prod`:

```env
ELYRA_ENABLE_TLS=1
ELYRA_DOMAIN=mail.example.com
WEB_BIND=127.0.0.1
```

3. Deploy with the TLS profile:

```sh
./infrastructure/deployment/deploy.sh --tls
```

Caddy listens on `80` and `443`; Nginx and the API stay private. Once the
certificate is issued, the public endpoints are:

- Website: `https://mail.example.com`
- API: `https://mail.example.com/api`
- Health: `https://mail.example.com/api/health`

If certificate issuance fails, confirm DNS resolves to the server and ports
80/443 are open, then check `docker compose ... logs caddy`.

## 6. Outbound mail relay

The relay lets registered Elyra users send to any external address even when
they have no local SMTP account. It is optional. Leave `ELYRA_SMTP_HOST` empty
to disable it; the desktop app then uses the user's own mail account.

Set the following in `.env.prod`:

```env
ELYRA_SMTP_HOST=smtp.example.com
ELYRA_SMTP_PORT=587
ELYRA_SMTP_SECURITY=starttls
ELYRA_SMTP_USERNAME=no-reply@example.com
ELYRA_SMTP_PASSWORD=app-password
ELYRA_SMTP_FROM=no-reply@example.com
ELYRA_SMTP_FROM_NAME=Elyra
```

- Use `ELYRA_SMTP_SECURITY=tls` with port `465` for implicit TLS.
- Use a dedicated mailbox or transactional provider, not a personal inbox.
- Publish SPF, DKIM, and DMARC records for the sending domain. Without them
  external providers will reject or spam-folder the messages.
- Gmail requires an App Password (2-Step Verification must be enabled).
  Transactional providers such as Resend, Brevo, Mailgun, or Postmark also work.
- The server never stores user IMAP passwords. Only the relay credentials live
  in `.env.prod`.
- Messages are sent from `ELYRA_SMTP_FROM` with `Reply-To` set to the signed-in
  user's Elyra email, so recipients can reply to the author.
- Add rate limiting and monitoring before exposing the relay publicly. Any
  registered user can send through it.

Verify the relay after signing in from the desktop app:

```sh
curl -fsS https://mail.example.com/api/v1/mail/status \
  -H "Authorization: Bearer <access-token>"
```

## 7. Point the desktop app at the server

In Elyra: **Settings → Elyra Cloud → Server URL**, enter
`https://mail.example.com/api`, click **Save server**, then **Connect account**
and register. The first registered user is a normal account; there is no
separate admin role yet.

For release installers, set the GitHub repository variable
`ELYRA_DEFAULT_API_URL=https://mail.example.com/api`. The release workflow bakes
it into the desktop build, so new users only need to register.

## 8. Automatic deployment from GitHub Actions

The `Deploy server` workflow runs on every push to `main` that touches
`server/`, `web/`, or `infrastructure/deployment/`, and can also be started
manually. It is disabled until you opt in.

1. Create a deploy key on the server:

```sh
ssh-keygen -t ed25519 -C "elyra-deploy" -f ~/.ssh/elyra_deploy -N ""
cat ~/.ssh/elyra_deploy.pub >> ~/.ssh/authorized_keys
cat ~/.ssh/elyra_deploy   # copy the private key
```

2. Add repository secrets in **GitHub → Settings → Secrets and variables →
   Actions**:

| Kind | Name | Value |
| --- | --- | --- |
| Secret | `ELYRA_DEPLOY_HOST` | Server IP or hostname |
| Secret | `ELYRA_DEPLOY_USER` | SSH user, for example `ubuntu` |
| Secret | `ELYRA_DEPLOY_KEY` | Contents of the private deploy key |
| Secret | `ELYRA_DEPLOY_PORT` | SSH port, optional, default `22` |
| Secret | `ELYRA_DEPLOY_PATH` | Absolute path, for example `/opt/elyra` |
| Variable | `ELYRA_DEPLOY_ENABLED` | `true` to enable the workflow |
| Variable | `ELYRA_DEFAULT_API_URL` | `https://mail.example.com/api` |

3. Make sure the deploy user can run Docker without `sudo` and owns the checkout
   (or can write to it). The workflow runs `git reset --hard origin/main` and
   then `deploy.sh --backup`.

The workflow uses `appleboy/ssh-action` and never prints secrets. If a deploy
fails, the previous containers keep running until the new build succeeds.

## 9. Backups and maintenance

Manual backup:

```sh
./infrastructure/deployment/deploy.sh --backup
# or
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

Copy backups off the server (object storage or another host) and test restores
periodically.

Update:

```sh
cd /opt/elyra
git pull --ff-only
./infrastructure/deployment/deploy.sh --backup
```

The server runs `CREATE TABLE IF NOT EXISTS` migrations on startup. Destructive
schema changes are not automated; review `server/src/database.rs` before
upgrading across major versions.

## 10. Troubleshooting

| Symptom | Check |
| --- | --- |
| `POSTGRES_PASSWORD must be set` | `.env.prod` is missing or not passed to Compose |
| API unhealthy | `docker compose ... logs api`, `DATABASE_URL`, PostgreSQL health |
| Website loads but API 502 | API container is down; Nginx proxies `/api/` to `api:8787` |
| Caddy certificate error | DNS points at the server, ports 80/443 open, `logs caddy` |
| Mail send returns `mail_not_configured` | `ELYRA_SMTP_HOST` is empty on the server |
| Mail send returns `mail_delivery_failed` | SMTP credentials, SPF/DKIM/DMARC, provider limits |
| Desktop cannot sign in | Server URL must end in `/api` and use HTTPS in production |
| 401 after server restart | `ELYRA_JWT_SECRET` changed; sign in again |

Useful commands:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml ps
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml logs --tail=100 api
curl -fsS https://mail.example.com/api/health
```
