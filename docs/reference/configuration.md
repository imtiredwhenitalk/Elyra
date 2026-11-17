# Configuration reference

Every runtime, build-time, and deployment variable used by Elyra 0.3.0, in one
place.

Legend: **Required** means the component fails or the feature does not work
without it. Values are case-sensitive.

## Desktop API URL

The desktop resolves the Elyra server base URL in this order:

1. `ELYRA_API_URL` environment variable (process environment).
2. SQLite `settings.api_url`, set through **Settings → Elyra Cloud → Server URL**.
3. `ELYRA_DEFAULT_API_URL`, compiled into the installer at build time.
4. `http://127.0.0.1:8787`.

Trailing slashes are stripped. The URL must start with `http://` or `https://`.
Production URLs must include `/api`, for example
`https://mail.example.com/api`.

| Variable | Scope | Required | Notes |
| --- | --- | --- | --- |
| `ELYRA_API_URL` | Desktop runtime | No | Overrides the saved setting; useful for development |
| `ELYRA_DEFAULT_API_URL` | Build time | No | Baked into release installers by the release workflow |
| `settings.api_url` | SQLite setting | No | Managed by the app; not an environment variable |

## Desktop SMTP fallback

When compose has no selected account, the desktop tries environment SMTP before
the cloud relay. All four required values must be present for this path to work.

| Variable | Required for fallback | Default | Notes |
| --- | --- | --- | --- |
| `ELYRA_SMTP_FROM` | Yes | — | Sender address; must parse as an email |
| `ELYRA_SMTP_HOST` | Yes | — | SMTP host |
| `ELYRA_SMTP_USERNAME` | Yes | — | SMTP username |
| `ELYRA_SMTP_PASSWORD` | Yes | — | SMTP password or app password |
| `ELYRA_SMTP_PORT` | No | `587` | Port |

The desktop environment fallback uses STARTTLS (`starttls_relay`). For implicit
TLS, configure a mail account instead. The sender display name defaults to
`Alex Smith` when `ELYRA_SMTP_FROM` has no display name.

## Desktop settings stored in SQLite

These are key/value rows in the local `settings` table. They are managed by the
UI, not environment variables.

| Key | Default | Purpose |
| --- | --- | --- |
| `theme` | `light` | `light`, `dark`, `midnight`, `oled` |
| `accent` | `moss` | `moss`, `ocean`, `ember`, `violet` |
| `density` | `comfortable` | `comfortable`, `compact` |
| `display_name` | `Alex Smith` | Local profile name |
| `domain_handle` | `alex@elyra.com` | Display-only Elyra handle prototype |
| `signature` | `Sent from Elyra` | Compose prefill |
| `local_only` | `true` | Local-first preference |
| `desktop_notifications` | `true` | Stored notification preference |
| `reduce_motion` | `false` | Disable non-essential animation |
| `auto_priority` | `true` | Apply local priority suggestions at import; `false` uses `priority_default` |
| `priority_default` | `normal` | `low`, `normal`, `high`, or `urgent` starting priority when suggestions are off |
| `blocked_sender_action` | `spam` | Destination for mail matched by a sender block rule: `spam` or `trash` |
| `api_url` | unset | Saved Elyra server base URL |
| `last_sync_at` | unset | Last successful account sync timestamp |
| `external_scanning` | `true` | Set to `false` to disable VirusTotal and Google Safe Browsing calls |
| `cloud_session` | legacy | Migrated to the keychain on first read, then deleted |

Browser preview also uses `localStorage` keys:

- `elyra-onboarding-skipped` — onboarding modal suppression.
- `elyra-security-preferences` — Security Lab preferences.
- `elyra-fallback:mailbox-v2` — preview mailbox data.
- `elyra-fallback:settings-v1` — preview settings.
- `elyra-fallback:message-security-v1` — preview Shield results.
- `elyra-fallback:crypto-keys-v1` — preview identity keys (no private material).
- `elyra-fallback:crypto-contacts-v1` — preview contact public keys.
- `elyra-fallback:integrations-v1` — preview integration status. Preview API
  keys are kept in memory only and are not written to `localStorage`.

## Keychain entries

Service name: `com.elyra.desktop`.

| Entry | Contents |
| --- | --- |
| `cloud_session` | Elyra Cloud session JSON, including the access token |
| `imap-account-<id>` | Mail account password |
| `integration-virustotal` | VirusTotal API key |
| `integration-google_safe_browsing` | Google Safe Browsing API key |
| `crypto-key-<id>` | Base64 Ed25519 identity private key |

These are not configurable through environment variables.

## Server

Loaded from the process environment and, in development, from `server/.env` via
`dotenvy`.

| Variable | Required | Default | Notes |
| --- | --- | --- | --- |
| `DATABASE_URL` | Yes | — | PostgreSQL connection string; pool max 10 |
| `ELYRA_JWT_SECRET` | Yes | — | At least 32 characters; server refuses to start otherwise |
| `ELYRA_SERVER_ADDR` | No | `127.0.0.1:8787` | Docker production sets `0.0.0.0:8787` |
| `ELYRA_ACCESS_TOKEN_TTL_HOURS` | No | `24` | JWT lifetime in hours |
| `RUST_LOG` | No | `elyra_server=info,tower_http=info` | Tracing filter |
| `ELYRA_SMTP_HOST` | No | empty | Empty disables the relay |
| `ELYRA_SMTP_PORT` | No | `587` for STARTTLS, `465` for TLS | SMTP port |
| `ELYRA_SMTP_SECURITY` | No | `starttls` | `tls` or `starttls`; anything else becomes `starttls` |
| `ELYRA_SMTP_USERNAME` | No | empty | Relay username |
| `ELYRA_SMTP_PASSWORD` | No | empty | Relay password |
| `ELYRA_SMTP_FROM` | When relay is on | — | Must be a valid mailbox; missing/invalid fails startup |
| `ELYRA_SMTP_FROM_NAME` | No | `Elyra` | Display name on relayed mail |

### Relay startup rules

- `ELYRA_SMTP_HOST` empty or whitespace → relay disabled, no error.
- `ELYRA_SMTP_HOST` set, `ELYRA_SMTP_FROM` missing → startup error.
- `ELYRA_SMTP_HOST` set, `ELYRA_SMTP_FROM` invalid → startup error.
- `ELYRA_SMTP_SECURITY=tls` → implicit TLS.
- Any other security value → STARTTLS.

## Production deployment

Set in `infrastructure/deployment/.env.prod` (copy from `.env.prod.example`).

| Variable | Required | Default | Notes |
| --- | --- | --- | --- |
| `POSTGRES_DB` | No | `elyra` | Database name |
| `POSTGRES_USER` | No | `elyra` | Database user |
| `POSTGRES_PASSWORD` | Yes | — | Compose fails if missing |
| `REDIS_PASSWORD` | Yes | — | Compose fails if missing; Redis requires auth |
| `ELYRA_JWT_SECRET` | Yes | — | At least 32 characters |
| `WEB_BIND` | No | `0.0.0.0` | Use `127.0.0.1` with Caddy |
| `WEB_PORT` | No | `8080` | Host port for the website |
| `RUST_LOG` | No | `elyra_server=info,tower_http=info` | API logging |
| `ELYRA_ENABLE_TLS` | No | `0` | `1` enables the Caddy profile without `--tls` |
| `ELYRA_DOMAIN` | With TLS | — | Domain for Caddy; required by the `tls` profile |
| `ELYRA_SMTP_*` | No | see server | Relay configuration |

### deploy.sh

| Variable | Default | Notes |
| --- | --- | --- |
| `ELYRA_ENV_FILE` | `infrastructure/deployment/.env.prod` | Alternative env file path |
| `ELYRA_ENABLE_TLS` | value from env file or `0` | Same as `--tls` |

CLI flags: `--tls`, `--no-build`, `--backup`, `--check`, `--help`.

## Development Docker stack

Set in `infrastructure/docker/.env` (copy from `.env.example`).

| Variable | Default | Notes |
| --- | --- | --- |
| `POSTGRES_DB` | `elyra` | Database name |
| `POSTGRES_USER` | `elyra` | Database user |
| `POSTGRES_PASSWORD` | `change-me-local` | Local only |
| `POSTGRES_PORT` | `5432` | Host port |
| `REDIS_PORT` | `6379` | Host port |
| `MAILPIT_SMTP_PORT` | `1025` | SMTP capture port |
| `MAILPIT_WEB_PORT` | `8025` | Mailpit UI |

## Build-time and CI variables

| Variable | Where | Purpose |
| --- | --- | --- |
| `ELYRA_DEFAULT_API_URL` | GitHub repository variable | Baked into release installers; also read by the release workflow |
| `ELYRA_DEPLOY_ENABLED` | GitHub repository variable | Must be exactly `true` to enable the SSH deploy workflow |
| `ELYRA_DEPLOY_HOST` | GitHub secret | Server hostname or IP |
| `ELYRA_DEPLOY_USER` | GitHub secret | SSH user |
| `ELYRA_DEPLOY_KEY` | GitHub secret | Private deploy key |
| `ELYRA_DEPLOY_PORT` | GitHub secret | SSH port, default `22` |
| `ELYRA_DEPLOY_PATH` | GitHub secret | Absolute checkout path, for example `/opt/elyra` |

## Launcher

| Variable | Default | Purpose |
| --- | --- | --- |
| `ELYRA_DOWNLOAD_URL` | `https://github.com/imtiredwhenitalk/Elyra/releases/latest/download/Elyra-latest-x64.msi` | MSI URL when `--url` is not passed |
| `ELYRA_MSI_SHA256` | unset | Expected checksum when `--sha256` is not passed |

The launcher also accepts `--url`, `--sha256`, `--msi`, `--silent`, and
`--no-launch`. See [Getting started](../getting-started.md#option-c-portable-launcher).

## Website

The website is static. It has no runtime environment variables.

- GitHub Pages serves `web/` directly.
- The production Nginx container serves `web/` and proxies `/api/` to
  `api:8787/`.
- `location = /health` proxies to `api:8787/health`.

## Security notes

- Never commit `.env`, `.env.prod`, or real secrets.
- `.env.prod` should be mode `600` and owned by the deploy user.
- Changing `ELYRA_JWT_SECRET` invalidates all access tokens.
- Changing `POSTGRES_PASSWORD` requires updating `DATABASE_URL` or the Compose
  environment consistently.
- Rotate relay credentials if they are exposed.
- Environment variables are visible to the process and may appear in crash dumps
  or container inspection. Prefer Docker secrets or an external secret manager
  for higher-assurance deployments.

## Related pages

- [Server and API](../server.md) — behavior behind the server variables.
- [Deployment](../deployment.md) — production usage.
- [Getting started](../getting-started.md) — user-facing setup.
- [Troubleshooting](../troubleshooting.md) — configuration symptoms.
