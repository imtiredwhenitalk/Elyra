> 🌐 **English** · [Українська](../ua/troubleshooting.md)

# Troubleshooting and FAQ

Use this page to diagnose common problems. Each section lists the symptom, the
likely cause, and the fix. When a page and the code disagree, the code wins;
source-of-truth files are listed in [docs/README.md](README.md).

## Installation

### Windows SmartScreen or Defender warns about the installer

Elyra 0.4.0 is not code-signed with an EV certificate, so Windows may show
"Unknown publisher" or a SmartScreen prompt.

1. Confirm the file came from the official
   [releases page](https://github.com/imtiredwhenitalk/Elyra/releases/latest).
2. Verify the SHA-256 checksum:

   ```powershell
   Get-FileHash .\Elyra-latest-x64.msi -Algorithm SHA256
   Get-Content .\Elyra-latest-x64.msi.sha256
   ```

3. If the hashes match, choose **More info → Run anyway**. If they do not match,
   delete the file and download it again.

Never bypass the warning for a file whose checksum you cannot verify.

### The MSI asks for administrator approval

`Elyra-latest-x64.msi` is a per-machine Windows Installer package and may request
elevation. If you cannot elevate, use `Elyra-latest-x64-setup.exe`, which
supports a per-user install mode.

### The launcher cannot download the MSI

- Check that GitHub Releases is reachable from your network. The launcher
  retries up to three times by default (`--retries <n>` changes this).
- Pass a local file instead: `Elyra-Launcher-x64.exe --msi C:\path\Elyra-latest-x64.msi`.
- Pass a mirror URL: `Elyra-Launcher-x64.exe --url https://example.com/Elyra-latest-x64.msi`.
  Plain HTTP is rejected unless you add `--allow-insecure-http`.
- The launcher writes downloads to `%TEMP%\elyra-installer\` and resumes from a
  `.part` file. Run `Elyra-Launcher-x64.exe cleanup` to remove stale transfers.
- If a checksum is published but does not match, the launcher refuses to install.
  Re-download the MSI and the `.sha256` sidecar together.
- Add `--require-checksum` if you want the launcher to refuse any installer
  without a published digest.

### `msiexec` fails during install

Run the MSI manually to see the Windows Installer error:

```powershell
msiexec /i .\Elyra-latest-x64.msi /passive /norestart
```

Common causes are an existing installation in a bad state, a pending reboot, or
group policy. Uninstall the previous version first, reboot, and try again.

### Uninstall did not delete my mail

That is intentional. Uninstalling keeps the local mailbox database so an
accidental uninstall does not destroy mail. Use the launcher's opt-in cleanup
to remove everything:

```powershell
.\Elyra-Launcher-x64.exe uninstall --backup-data C:\Elyra-backup --remove-data
```

`--remove-data` deletes `%LOCALAPPDATA%\com.elyra.desktop` and the Elyra entries
in **Windows Credential Manager**. Preview the actions first with
`--dry-run`, and keep the `--backup-data` folder until you are sure you no
longer need the mail. Without the flag, remove the folder manually.

## First run and sign-in

### The onboarding modal keeps appearing

The "Continue offline" choice stores `elyra-onboarding-skipped` in the webview's
`localStorage`. If storage is cleared or blocked, the flag is lost. Sign in or
choose **Continue offline** again. In a desktop build, clearing app data also
removes the flag.

### Registration fails with a password error

The server requires at least 12 characters. The display name must be non-empty
and at most 120 characters. The email must contain `@` and be at most 320
characters.

### Registration fails with `409 conflict`

That normalized email is already registered. Sign in instead, or use a
different address. Letter case and surrounding whitespace are ignored, so
`User@example.com` and `user@example.com` are the same account. On upgrades,
duplicate legacy rows must be merged before the server can create its unique
index.

### Sign-in fails with `invalid credentials`

- Check the password.
- Check that you are pointing at the server that holds the account. A local
  server and a production server have separate databases.
- If the server's `ELYRA_JWT_SECRET` changed, existing tokens are invalid, but
  passwords still work; sign in again.

### The app cannot reach the server

- Open **Settings → Elyra Cloud** and check the server URL.
- The URL must include the API path in production:
  `https://mail.example.com/api`, with no trailing slash.
- Verify the server:

  ```powershell
  curl.exe -fsS https://mail.example.com/api/health
  ```

- Check the desktop override: an `ELYRA_API_URL` environment variable takes
  precedence over the saved setting.
- A local server defaults to `http://127.0.0.1:8787` and must be running.

### I am signed out after a while

Access tokens expire after `ELYRA_ACCESS_TOKEN_TTL_HOURS` (24 hours by default).
There is no refresh token in 0.4.0, so sign in again. Changing
`ELYRA_JWT_SECRET` invalidates every token immediately.

## Mail accounts and IMAP

### Gmail rejects my password

Gmail requires 2-Step Verification and an **App Password** for password-based
IMAP. A normal account password is rejected. Create an app password in your
Google account and paste it into Elyra, or configure
`ELYRA_GOOGLE_CLIENT_ID` and use OAuth 2.0 sign-in. See
[Provider OAuth](oauth.md).

### Outlook or Microsoft 365 rejects the login

Many tenants disable basic IMAP authentication or require tenant consent. Check
the tenant's IMAP setting; if allowed, use an app password, or configure
`ELYRA_MICROSOFT_CLIENT_ID` and use OAuth 2.0 sign-in. An administrator may still
need to grant the requested IMAP scope. See [Provider OAuth](oauth.md).

### iCloud or Yahoo rejects the login

Both require app-specific passwords. Generate one in the provider's account
security settings and use that instead of the main password.

### Proton Mail cannot connect

Proton does not expose standard IMAP directly. Install **Proton Mail Bridge**,
sign in to it, and keep it running. Elyra's Proton preset uses `127.0.0.1`; enter
the Bridge IMAP/SMTP credentials, not your Proton account password.

### Test connection fails but the settings look right

- Use TLS on port 993 for IMAP and 465 for SMTP, or STARTTLS on 143/587 where the
  provider supports it. Plaintext is not supported.
- Some networks block IMAP/SMTP ports. Try another network.
- Corporate proxies and TLS inspection can break certificate validation.
- Verify the username. Some providers require the full address, others do not.

### Sync imports nothing

- Click **Sync** on the specific account. The header refresh button does not
  contact IMAP.
- Only the provider's **INBOX** is synced in 0.4.0. Sent, Drafts, Archive, Spam,
  and Trash are not pulled.
- Check the account row for a sync error. Diagnostics does not show per-account
  errors.
- The desktop requests 40 messages by default. Use `sync_email_account` with a
  higher limit (up to 200) if you need more, but the UI uses 40.

### Sync created duplicates

Sync upserts by `(account_id, provider_id)`, so repeated syncs should not
duplicate. Duplicates usually mean the same mailbox was added twice as two
separate accounts. Remove one account.

### My read or starred state was overwritten

Local `is_read` and `is_starred` are preserved across syncs. If state looks
wrong, confirm you are not editing the same message through two accounts.

### Removing an account deleted local messages

That is the documented behavior. Removing an account deletes its keychain entry
and its locally cached messages. Export the mailbox first if you need a copy.

## Sending

### Send fails with "SMTP setting ... is not configured"

No sending account is selected and the `ELYRA_SMTP_*` environment fallback is
incomplete. Either select a mail account in compose, configure all of
`ELYRA_SMTP_FROM`, `ELYRA_SMTP_HOST`, `ELYRA_SMTP_USERNAME`, and
`ELYRA_SMTP_PASSWORD`, or sign in to Elyra Cloud so the relay can be used.

### Send fails with "Elyra Cloud relay failed"

- Confirm you are signed in and the token has not expired.
- Check `GET {api}/v1/mail/status`; if `configured` is false, the server operator
  has not enabled the relay.
- The relay returns `mail_not_configured` (503) or `mail_delivery_failed` (502)
  with a message. Ask the operator to check server logs and SMTP credentials.

### Mail is sent but the recipient sees the relay address

That is expected for relayed mail. The server sends from `ELYRA_SMTP_FROM` and
sets `Reply-To` to your Elyra account email. Replies should return to you if the
server and receiving provider honor `Reply-To`.

### A failed send is missing from Sent

Failed sends are not recorded in **Sent**. In 0.4.0 they are kept in the local
outbox instead, so you can retry or discard them from **Settings → Sync &
offline**. Flush the outbox after fixing the account or relay configuration.

### The outbox shows blocked messages

A message is marked blocked after five failed attempts. Check the stored error,
fix the underlying SMTP or relay problem, then select **Retry** to reset the
attempt counter. **Discard** permanently removes the queued message from this
device.

### Work offline is on but messages were sent

Turn off **Work offline** and flush the outbox only when you intend to send.
While the switch is on, `flush_outgoing_queue` skips every queued message.

### Drafts are not on the provider

Drafts are local SQLite rows in 0.4.0. They are not synced to IMAP.

## Notifications and cloud features

### The bell shows nothing

Notifications require a signed-in Elyra Cloud session and a reachable server.
The list returns up to 100 notifications for the current account. Other users'
notifications are never visible.

### Sending a notification returns `404 not_found`

The recipient email must belong to an existing Elyra account on the same server.
Notifications are in-app records, not email.

### Desktop notifications do not appear when the app is closed

The **Desktop notifications** setting stores a preference only. In 0.4.0, cloud
notifications appear in the in-app popover. There is no background push channel.

### Signing out did not delete my mail

Sign out deletes the keychain cloud session and the in-memory session. It does
not touch local mail or mail-account passwords.

## Data and settings

### Where is the database?

`%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3` on Windows. The exact path is
shown in **Settings → Diagnostics**.

### Settings do not persist

Desktop builds store settings in SQLite. If the database cannot be opened, the
UI may fall back to browser storage or defaults. Check Diagnostics for the
database path, size, and SQLite version, and check file permissions in
`%LOCALAPPDATA%\com.elyra.desktop`.

### "Database is locked" or the app hangs on startup

Close every Elyra window, then check for a leftover process in Task Manager.
WAL mode allows concurrent readers, but two writers still conflict. If the file
is corrupted, back up `elyra.sqlite3` plus any `-wal`/`-shm` files and start with
a fresh database.

### Reset demo mailbox destroyed real messages

**Reset demo mailbox** deletes attachments and emails before reseeding. It is
destructive. Restore from a backup; there is no undo.

### Export contains sensitive data

`export_mailbox` returns message bodies and metadata as JSON. Store the export
like you would store the mailbox itself.

### Security Lab says "Decryption failed"

That is the expected result for a wrong key, a tampered payload, invalid base64,
or a truncated payload. Generate a key, encrypt, then decrypt with the same key.
Keys are held in memory only and are lost when the app restarts.

## Browser preview

### Some buttons do nothing in the browser

The browser preview uses a `localStorage` fallback and intentionally does not
exercise Rust, SQLite, keychain, IMAP, SMTP, or crypto. Use `npm.cmd run tauri dev`
for the real desktop runtime.

### The preview shows different data than the app

The preview uses `elyra-fallback:mailbox-v2` and `elyra-fallback:settings-v1` in
`localStorage`. The desktop app uses SQLite. They are separate stores.

## Build and development

### `npm` is blocked by a PowerShell execution policy

Use the command shim explicitly:

```powershell
npm.cmd install --prefix apps/desktop
npm.cmd run check
npm.cmd run build
```

### `cargo` or `rustc` is not found

Install Rust from <https://rustup.rs>, then reopen the terminal. On Windows,
install the Microsoft C++ Build Tools with the **Desktop development with C++**
workload and the `x86_64-pc-windows-msvc` target.

### The desktop build fails with WebView2 errors

Install the WebView2 Runtime. It ships with current Windows 10/11 but may be
missing on stripped or older images.

### `cargo check -p tauri-app` fails but the server builds

The desktop crate has Windows-only dependencies (including `keyring` with the
`windows-native` feature). Use a Windows toolchain for the desktop crate. CI
builds it on `windows-latest`.

### `Cargo.lock` looks out of date

The lockfile may lag behind workspace dependencies. CI and Docker resolve
without `--locked`. To regenerate:

```powershell
cargo update -w
git add Cargo.lock
```

### The release workflow did not publish assets

- The workflow triggers on tags matching `v*` or manual dispatch.
- Check the **Collect release assets** step for missing bundle paths.
- Expected assets: `Elyra-latest-x64.msi`, `Elyra-latest-x64-setup.exe`,
  `Elyra-Launcher-x64.exe`, `Elyra-latest-x64.msi.sha256`.
- A failing Rust build stops the release. Check the workflow log first.

### The website did not update

GitHub Pages deploys `web/` on pushes to `main`. Check the **Deploy GitHub
Pages** workflow and the Pages environment. Browser caching uses short-lived
headers; hard-refresh if needed.

## Server and deployment

### The server exits at startup with `ELYRA_JWT_SECRET is not configured`

Set a secret of at least 32 characters. The server refuses to start without one.
Generate it with `openssl rand -hex 32`.

### The server exits with `ELYRA_SMTP_FROM is required`

`ELYRA_SMTP_HOST` is set, which enables the relay, but the sender address is
missing or invalid. Set a valid `ELYRA_SMTP_FROM` or clear `ELYRA_SMTP_HOST` to
disable the relay.

### The API cannot connect to PostgreSQL

- Check `DATABASE_URL`.
- In Compose, check `POSTGRES_PASSWORD`, `POSTGRES_USER`, and `POSTGRES_DB`.
- Wait for the PostgreSQL health check; the API depends on it.
- Inspect logs:

  ```sh
  docker compose --env-file infrastructure/deployment/.env.prod \
    -f infrastructure/deployment/docker-compose.prod.yml logs --tail=100 api
  ```

### `deploy.sh` fails immediately

- Confirm Docker Engine 24+ and the Compose plugin are installed.
- Confirm `.env.prod` exists and is readable.
- Run `./infrastructure/deployment/deploy.sh --check` to validate configuration
  without building.

### HTTPS does not work with Caddy

- The DNS `A`/`AAAA` record must already point at the server.
- Ports 80 and 443 must be reachable from the internet.
- `ELYRA_DOMAIN` must be set and match the certificate name.
- Check Caddy logs:

  ```sh
  docker compose --env-file infrastructure/deployment/.env.prod \
    -f infrastructure/deployment/docker-compose.prod.yml --profile tls logs --tail=100 caddy
  ```

### The desktop gets 404s from the server

The public API base must include `/api` and have no trailing slash:
`https://mail.example.com/api`. Nginx proxies `location /api/` to
`http://api:8787/` and strips the prefix. A URL without `/api` hits the static
site instead.

### The relay returns `mail_delivery_failed`

Check the SMTP host, port, security mode, username, and password. Use `tls` on
465 or `starttls` on 587. Confirm the provider allows the sending address and
that SPF/DKIM/DMARC are configured. Inspect API logs for the underlying error.

### Registration is open to anyone

That is a known 0.4.0 limitation. Add a reverse-proxy allowlist, an invite code,
or admin approval before exposing the server publicly. See
[Security → Known gaps](security.md#known-gaps-and-follow-ups).

### The relay can be abused

There is no per-user rate limit or quota in 0.4.0. Disable `ELYRA_SMTP_HOST`
until you add rate limiting, or restrict who can register.

### Backups fail

Run the documented `pg_dump` command and confirm the `backups/` directory is
writable. Copy backups off the server and test a restore periodically. New dumps
use `pg_dump --clean --if-exists`; restore them into an empty database with
`psql -v ON_ERROR_STOP=1`. If an older dump fails with `relation already exists`,
start from an empty database instead. See
[Deployment → Backups](deployment.md#backups) and the
[production verification checklist](deploy/VERIFY.md).

### Database-backed server tests skip

`cargo test -p elyra-server` runs pure unit tests without PostgreSQL, but the
database-backed API and schema tests skip unless `ELYRA_TEST_DATABASE_URL` points
to a disposable database. Set it explicitly to make those failures visible. See
[Configuration → Server tests](reference/configuration.md#server-tests).

### Auto-deploy did not run

The `Deploy server` workflow is disabled unless the repository variable
`ELYRA_DEPLOY_ENABLED` is exactly `true`, and the `ELYRA_DEPLOY_*` secrets must
be configured. It also only triggers on matching paths or manual dispatch.

## Diagnostics checklist

When reporting a problem, include:

1. Elyra version from **Settings → Diagnostics**.
2. Windows or Linux version and architecture.
3. The exact error message or toast text.
4. **Settings → Diagnostics & privacy → Export diagnostic report**. Events are
   redacted before they are saved, but review the file before sharing it: it
   still contains local counts, paths and message metadata.
5. Whether the problem happens in the desktop app, browser preview, or both.
6. For server issues: `docker compose ps`, recent API logs, and the output of
   `curl -fsS https://<domain>/api/health`.
7. For IMAP issues: provider, host, port, and security mode. Never include your
   password.

## Related pages

- [Getting started](getting-started.md) — install and first run.
- [User guide](user-guide.md) — expected behavior for each control.
- [Server and API](server.md) — routes, validation, and error codes.
- [Deployment](deployment.md) — production stack and HTTPS.
- [Security](security.md) — what is and is not protected.
- [Development](development.md) — build and release commands.
