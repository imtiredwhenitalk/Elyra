# Elyra

**Elyra** is a privacy-focused email client that brings your inbox, security, and
communication into one calm desktop workspace. Mailbox logic runs in Rust, data
stays on your device by default, and the Windows build ships as an MSI or a
per-user NSIS installer.

> **Your email. Your privacy. Your control.**

## Download

The newest Windows build is published on the
[releases page](https://github.com/imtiredwhenitalk/Elyra/releases/latest):

| Asset | Purpose |
| --- | --- |
| `Elyra-latest-x64.msi` | Standard Windows Installer package (per-machine, may request administrator approval) |
| `Elyra-latest-x64-setup.exe` | NSIS setup with a per-user install mode |
| `Elyra-Launcher-x64.exe` | Portable launcher that downloads, verifies, installs, and starts Elyra |
| `Elyra-latest-x64.msi.sha256` | SHA-256 checksum used by the launcher |

The launcher accepts `--msi <path>` for an already-downloaded installer and
automatically fetches `<url>.sha256` when installing from a URL.

## What shipped in 0.3.0

* **OS keychain secret storage** — Elyra Cloud sessions and IMAP/SMTP passwords
  are stored in the Windows Credential Manager through `keyring-rs`. The legacy
  SQLite `settings.cloud_session` row is migrated to the keychain and removed.
* **IMAP provider sync** — add Gmail, Outlook/Microsoft 365, iCloud, Yahoo,
  Fastmail, Proton Mail Bridge, or a custom server; test IMAP and SMTP; and pull
  the newest INBOX messages into the Rust mailbox. Local read and starred state
  is preserved across syncs.
* **Per-account sending** — compose selects a sending account and uses its SMTP
  settings plus keychain password, with the environment variables as fallback.
* **Account UI** — dynamic connected-account list, add/edit modal with provider
  presets, connection testing, per-account sync, sync-all, and keychain status in
  Diagnostics.

## What shipped in 0.3.0

* **Keychain + IMAP sync** — Gmail, Outlook, iCloud, Yahoo, Fastmail, Proton
  Bridge, and custom servers; IMAP/SMTP connection tests, INBOX sync, and
  per-account sending. Passwords live in the OS keychain.
* **First-run registration** — the app offers account registration on first
  launch, accepts a configurable Elyra server URL, and can continue offline.
* **Send to any address** — registered users without a local SMTP account send
  through the server's optional SMTP relay.
* **Server deployment** — a production Compose stack, one-command `deploy.sh`
  with health checks and backups, optional Caddy HTTPS, and an opt-in GitHub
  Actions SSH deploy workflow.

## What shipped in 0.2.0

* **Rust mailbox core** — SQLite schema migration, demo seeding, search, folder
  filters, statistics, drafts, bulk actions, snooze, attachments, sent-mail
  recording, and JSON export all run as Tauri commands.
* **Rust cloud bridge** — account registration, login, session refresh,
  notifications, and sync run through blocking `reqwest` calls. Access tokens
  stay inside the Rust/SQLite layer; the webview only receives a session view.
* **Diagnostics** — the app reports engine, Tauri, SQLite, database size, and
  changelog information from Rust.
* **Interface rebuild** — a single inline SVG icon sprite, four themes, four
  accents, density options, bulk selection, a command palette, a richer reading
  pane, and a redesigned settings/profile experience.
* **Windows delivery** — a release workflow builds MSI and NSIS bundles, renames
  assets to stable names, writes SHA-256 checksums, and publishes a GitHub
  Release. The launcher verifies the MSI magic bytes and checksum before running
  `msiexec`.
* **Public website** — a static marketing and download page under `web/`,
  deployed to GitHub Pages.

## Local data

The desktop database lives at `%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3` on Windows.
The exact path is shown in **Settings → Diagnostics**. SQLite uses WAL journaling,
foreign keys, and secure deletion. Cloud sessions and mailbox passwords live in
the OS keychain, not in the database.

## Development

Requirements: Node.js 20+, Rust stable, and the platform prerequisites for
Tauri 2. Mail can be sent through a configured account, or through
`ELYRA_SMTP_FROM`, `ELYRA_SMTP_HOST`, `ELYRA_SMTP_USERNAME`, and
`ELYRA_SMTP_PASSWORD` in the process environment.

```powershell
npm.cmd install --prefix apps/desktop
npm.cmd run check
npm.cmd run build        # Windows MSI + NSIS bundles
cargo test -p elyra-crypto -p elyra-server -p elyra-launcher
cargo check -p tauri-app
```

The frontend is static (`apps/desktop/src`), so no bundler is required. The
browser preview fallback in `apps/desktop/src/services/tauri.js` keeps the UI
usable without Tauri, while desktop builds always use the Rust commands.

### Server and shared packages

The Rust workspace also contains the reusable [`elyra-crypto`](packages/crypto)
package and the authenticated [`server`](server) service. The server uses
PostgreSQL for account metadata and sync cursors, Argon2id for password hashing,
and signed access tokens. It never receives local encryption keys.

Run `cargo run -p elyra-server` after starting PostgreSQL and configuring the
variables from [`server/.env.example`](server/.env.example). Shared TypeScript
contracts live in [`packages/types`](packages/types), and framework-neutral UI
primitives live in [`packages/ui`](packages/ui).

Registered desktop users can send to any external email address through the
optional server SMTP relay; without it, Elyra uses the user's own SMTP account.
Production deployment, HTTPS, SMTP configuration, backups, and GitHub Actions
auto-deploy are documented in [`docs/deploy/SERVER.md`](docs/deploy/SERVER.md).

## Project layout

* `apps/desktop` — Tauri desktop app, static UI, and Rust commands.
* `apps/launcher` — portable Windows installer/updater CLI.
* `packages/crypto` — authenticated encryption primitives.
* `server` — Axum API, PostgreSQL migrations, auth, mail relay, and sync cursors.
* `web` — static marketing and download site.
* `infrastructure` — Docker and deployment assets.
* `docs` — architecture, crypto, and security notes.

## Status

Elyra is under active development. The local mailbox, Rust crypto lab, cloud
auth/notifications, OS keychain secret storage, IMAP inbox sync, and Windows
delivery path are implemented. Gmail/Outlook OAuth with PKCE and scheduled
background sync are the next milestone. See [`task.md`](task.md) and
[`changes.md`](changes.md) for the live tracking log.
