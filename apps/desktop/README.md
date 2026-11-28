# Elyra desktop

The Elyra desktop client is a Tauri 2 application with a static HTML/CSS/JS
frontend and a Rust core. The webview renders the interface; mailbox queries,
cloud calls, diagnostics, cryptography, and SQLite access run in Rust.

## Layout

* `src/` — static frontend, styles, inline SVG sprite, and Tauri bridge.
* `src-tauri/src/` — Rust commands (`mailbox`, `providers`, `secrets`, `cloud`,
  `storage`, `security`, `crypto`, `diagnostics`).
* `src-tauri/icons/` — generated application icons.
* `services/sendemail.rs` — SMTP sending and sent-mail recording.

## Commands

```powershell
npm.cmd install
npm.cmd run check     # JavaScript syntax check
npm.cmd run tauri dev # development window
npm.cmd run build     # MSI + NSIS bundles
```

The frontend is loaded directly from `src/` (`frontendDist`), so there is no
bundler step. Browser preview falls back to `localStorage` through
`src/services/tauri.js`; desktop builds always invoke the Rust commands.

## Local data

`%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3` on Windows. Settings → Diagnostics shows the
resolved path, SQLite version, database size, and mailbox counts.

Cloud sessions and IMAP/SMTP passwords are stored in the Windows Credential
Manager through `keyring-rs`, not in SQLite. The Diagnostics panel reports
whether the keychain is reachable.

## Mail accounts and sync

Settings → Mail accounts adds Gmail, Outlook/Microsoft 365, iCloud, Yahoo,
Fastmail, Proton Mail Bridge, or a custom IMAP/SMTP server. Each account stores
metadata in SQLite and its password in the OS keychain. **Test connection**
checks IMAP and SMTP, and **Sync** pulls the newest INBOX messages into the Rust
mailbox while preserving local read and starred state. The refresh button syncs
all configured accounts.

## Sending mail

Compose selects a sending account; its SMTP settings and keychain password are
used for delivery. With no account selected, the environment fallback applies:
set `ELYRA_SMTP_FROM`, `ELYRA_SMTP_HOST`, `ELYRA_SMTP_USERNAME`,
`ELYRA_SMTP_PASSWORD`, and optionally `ELYRA_SMTP_PORT` before launching the
desktop process.

If neither an account nor the environment is configured and the user is signed
in, mail is sent through the Elyra server relay. The server operator enables it
with `ELYRA_SMTP_*`; see [`docs/en/deploy/SERVER.md`](../../docs/en/deploy/SERVER.md).

## Accounts and server URL

On first launch, Elyra offers account registration and a server URL field. The
URL is stored in SQLite `settings.api_url` and can be changed later under
Settings → Elyra Cloud. Resolution order is:

1. `ELYRA_API_URL` environment variable.
2. The saved server URL.
3. `ELYRA_DEFAULT_API_URL` compiled into the installer.
4. `http://127.0.0.1:8787`.

Release builds can bake the production URL into the installer by setting the
repository variable `ELYRA_DEFAULT_API_URL` before tagging a release.
