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
foreign keys, and secure deletion.

## Development

Requirements: Node.js 20+, Rust stable, and the platform prerequisites for
Tauri 2. Sending mail also needs `ELYRA_SMTP_FROM`, `ELYRA_SMTP_HOST`,
`ELYRA_SMTP_USERNAME`, and `ELYRA_SMTP_PASSWORD` in the process environment.

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

## Project layout

* `apps/desktop` — Tauri desktop app, static UI, and Rust commands.
* `apps/launcher` — portable Windows installer/updater CLI.
* `packages/crypto` — authenticated encryption primitives.
* `server` — Axum API, PostgreSQL migrations, auth, and sync cursors.
* `web` — static marketing and download site.
* `infrastructure` — Docker and deployment assets.
* `docs` — architecture, crypto, and security notes.

## Status

Elyra is under active development. The local mailbox, Rust crypto lab, cloud
auth/notifications, and Windows delivery path are implemented. Gmail/Outlook
OAuth with PKCE, IMAP fetch, OS keychain token storage, and background sync are
the next milestone. See [`task.md`](task.md) and [`changes.md`](changes.md) for
the live tracking log.
