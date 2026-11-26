# Production deployment verification

Use this checklist after deploying Elyra to a private server. It proves that the
stack is healthy, the API works through the public HTTPS endpoint, data survives
a restart, and the backup/restore path is usable.

Elyra needs **no source changes** for a custom domain or a private deployment.
Site-specific values live in `infrastructure/deployment/.env.prod`, in DNS, and
in the desktop **Settings → Elyra Cloud → Server URL** field. For installers, set
the GitHub repository variable `ELYRA_DEFAULT_API_URL`. Keep the checkout
unmodified so upgrades remain a plain `git pull`.

The full setup walkthrough is in [SERVER.md](SERVER.md); this page is the
verification and smoke-test companion.

## 1. Before deploying

- [ ] The checkout is clean and on the intended commit or tag.
- [ ] `.env.prod` exists, is mode `600`, and is not tracked by Git.
- [ ] `POSTGRES_PASSWORD`, `REDIS_PASSWORD`, and `ELYRA_JWT_SECRET` are unique
      random values; `ELYRA_JWT_SECRET` is at least 32 characters.
- [ ] `ELYRA_DOMAIN` points at this server (`A`/`AAAA` records) when TLS is on.
- [ ] The firewall allows SSH and, with Caddy, ports 80/443 only. PostgreSQL,
      Redis, and the API are not published.
- [ ] `WEB_BIND=127.0.0.1` when the `tls` profile is enabled.
- [ ] The SMTP relay settings are intentional, or `ELYRA_SMTP_HOST` is empty.
- [ ] A backup exists before any upgrade or configuration change.

Validate configuration without starting anything:

```sh
./infrastructure/deployment/deploy.sh --check
```

Run the automated tests (see [Automated tests](#2-automated-tests)). Then deploy,
for example:

```sh
./infrastructure/deployment/deploy.sh --backup --tls
```

## 2. Automated tests

Pure tests always run and need no services:

```sh
cargo test -p elyra-server
```

The database-backed API and schema tests use a disposable PostgreSQL database.
They skip themselves when no database is reachable, so a plain `cargo test`
never fails just because Docker is absent:

```sh
ELYRA_TEST_DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra_test \
  cargo test -p elyra-server
```

On Windows PowerShell:

```powershell
$env:ELYRA_TEST_DATABASE_URL = "postgres://elyra:elyra@127.0.0.1:5432/elyra_test"
cargo test -p elyra-server
```

`ELYRA_TEST_DATABASE_URL` is an explicit opt-in: connection or migration
failures then fail the run. `DATABASE_URL` is a fallback only when reachable.
The `Rust` CI workflow already runs `cargo test -p elyra-crypto -p
elyra-server -p elyra-launcher`, so these commands do not require a workflow
change; a CI service container can set `ELYRA_TEST_DATABASE_URL` to enable the
database cases.

Coverage:

| Area | Where |
| --- | --- |
| Auth (password hashing, JWT signing, expiry, wrong secret) | `server/src/auth.rs` |
| API input validation, bearer parsing, error mapping | `server/src/api.rs` |
| Register/login, sync cursor, mail relay fail-closed | `server/src/api.rs` (database-backed) |
| Notification privacy and read ownership | `server/src/api.rs` (database-backed) |
| Schema idempotency, uniqueness, upsert, cascade deletes | `server/tests/database_integration.rs` |
| SMTP config normalization and disabled-relay behavior | `server/src/mailer.rs` |

## 3. Public smoke tests

Set the public API base (it must end in `/api`) and confirm liveness:

```sh
BASE=https://mail.example.com/api
curl -fsS "$BASE/health"
# {"service":"elyra-server","status":"ok"}
```

Register a throwaway user with a non-routable address:

```sh
VERIFY_EMAIL="verify-$(date +%s)@example.invalid"
RESPONSE=$(curl -sS -X POST "$BASE/v1/auth/register" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$VERIFY_EMAIL\",\"password\":\"verification-passphrase\",\"display_name\":\"Verifier\"}")
printf '%s\n' "$RESPONSE"
```

Copy `access_token` from the response, or use `jq` if installed:

```sh
TOKEN=$(printf '%s' "$RESPONSE" | jq -r .access_token)
test -n "$TOKEN" && echo "token acquired"
```

Authenticated checks:

```sh
curl -fsS "$BASE/v1/users/me" -H "Authorization: Bearer $TOKEN"

curl -fsS -X PUT "$BASE/v1/sync/cursor" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"cursor":"verify-1"}'
curl -fsS "$BASE/v1/sync/cursor" -H "Authorization: Bearer $TOKEN"
# {"cursor":"verify-1","synced_at":"..."}

CREATED=$(curl -sS -X POST "$BASE/v1/notifications" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d "{\"recipient_email\":\"$VERIFY_EMAIL\",\"title\":\"Verify\",\"body\":\"Smoke test\"}")
printf '%s\n' "$CREATED"
NOTE_ID=$(printf '%s' "$CREATED" | jq -r .id)

curl -fsS "$BASE/v1/notifications" -H "Authorization: Bearer $TOKEN"
curl -sS -o /dev/null -w '%{http_code}\n' -X PATCH \
  "$BASE/v1/notifications/$NOTE_ID/read" -H "Authorization: Bearer $TOKEN"
# 204

curl -fsS "$BASE/v1/mail/status" -H "Authorization: Bearer $TOKEN"
```

Expected results:

- [ ] `/health` returns `status: ok` through the `/api` proxy.
- [ ] Registration returns a session and a `user.id`.
- [ ] `me` returns the same user id.
- [ ] Cursor round-trip persists the value and sets `synced_at`.
- [ ] Notification create returns `201`, list contains it, mark-read returns
      `204`.
- [ ] Missing or invalid bearer tokens return `401` (check below).
- [ ] `mail/status` reports `configured` matching your `.env.prod`.

Negative security checks:

```sh
curl -sS -o /dev/null -w '%{http_code}\n' "$BASE/v1/users/me"                    # 401
curl -sS -o /dev/null -w '%{http_code}\n' "$BASE/v1/users/me" \
  -H 'Authorization: Bearer not-a-jwt'                                           # 401
curl -sS -o /dev/null -w '%{http_code}\n' -X POST "$BASE/v1/auth/register" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$VERIFY_EMAIL\",\"password\":\"verification-passphrase\"}"    # 409
```

If the relay reports `configured: true`, send exactly one message to a mailbox
you control and confirm delivery, `Reply-To`, SPF/DKIM/DMARC results, and that
the 0.4.0 relay has no rate limit. Do not use the public verification user for
bulk sends.

## 4. HTTPS checks (TLS profile)

```sh
curl -fsSI https://mail.example.com/ | head -n 1
curl -sSI https://mail.example.com/ | grep -i strict-transport-security
echo | openssl s_client -servername mail.example.com -connect mail.example.com:443 2>/dev/null \
  | openssl x509 -noout -dates
curl -sSI http://mail.example.com/ | head -n 1
```

- [ ] HTTPS site and `/api/health` load with a valid certificate.
- [ ] Certificate dates are current and renewals are not failing in
      `docker compose ... logs caddy`.
- [ ] `Strict-Transport-Security` is present; plain HTTP redirects to HTTPS.
- [ ] The desktop app signs in with Server URL `https://mail.example.com/api`.

## 5. Persistence and restart

- [ ] Note a notification id or cursor value from step 3.
- [ ] Restart the API container:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml restart api
```

- [ ] `/health` returns to `ok`.
- [ ] The same `TOKEN` still authenticates and the saved cursor/notification is
      present. Tokens survive restarts while `ELYRA_JWT_SECRET` is unchanged.

## 6. Backup and restore drill

- [ ] `deploy.sh --backup` produces a non-empty gzipped dump in `backups/`.
- [ ] `gzip -t backups/<file>.sql.gz` succeeds.
- [ ] A copy exists off the server (object storage or another host).
- [ ] A restore was rehearsed on a staging instance using the procedure in
      [SERVER.md §9](SERVER.md#9-backups-and-maintenance), and sign-in plus data
      were verified afterwards.
- [ ] The rollback path is known: checkout the previous good tag, redeploy with
      `--backup`, and restore the matching database dump if the schema changed.

## 7. Clean up and sign off

Remove the throwaway user (substitute the database values from `.env.prod`):

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml \
  exec -T postgres psql -U elyra -d elyra \
  -c "DELETE FROM users WHERE email LIKE 'verify-%@example.invalid';"
```

- [ ] Verification user removed.
- [ ] Logs show no unexpected errors during the smoke tests.
- [ ] Monitoring covers `/health`, disk usage, and container restarts.
- [ ] Result recorded: date, commit/tag, domain, operator, and any follow-ups.

## Related pages

- [Server runbook](SERVER.md) — setup, secrets, TLS, backups, troubleshooting.
- [Deployment overview](../deployment.md) — stack and quick start.
- [Server and API](../server.md) — routes, payloads, and error codes.
- [Development](../development.md) — local environment and test procedures.
