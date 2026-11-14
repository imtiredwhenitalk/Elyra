# Elyra desktop

The Elyra desktop client is a Tauri 2 application with a static HTML/CSS/JS
frontend and a Rust core. The webview renders the interface; mailbox queries,
cloud calls, diagnostics, cryptography, and SQLite access run in Rust.

## Layout

* `src/` — static frontend, styles, inline SVG sprite, and Tauri bridge.
* `src-tauri/src/` — Rust commands (`mailbox`, `cloud`, `storage`, `security`,
  `crypto`, `diagnostics`).
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

## Sending mail

Set `ELYRA_SMTP_FROM`, `ELYRA_SMTP_HOST`, `ELYRA_SMTP_USERNAME`,
`ELYRA_SMTP_PASSWORD`, and optionally `ELYRA_SMTP_PORT` before launching the
desktop process.
