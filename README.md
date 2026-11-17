<div align="center">

<img src="web/favicon.svg" width="84" height="84" alt="Elyra" />

<h1>Elyra</h1>

<p><strong>Your inbox, back on your side.</strong></p>

<p>
  A quiet, local-first email workspace for Windows.<br />
  Mailbox logic runs in Rust, your mail lives in local SQLite,<br />
  and security checks run locally first.
</p>

<p>
  <a href="https://github.com/imtiredwhenitalk/Elyra/releases/latest"><img src="https://img.shields.io/github/v/release/imtiredwhenitalk/Elyra?style=flat-square&color=4d7a61&label=release" alt="Latest release" /></a>
  <a href="https://github.com/imtiredwhenitalk/Elyra/actions/workflows/rust.yml"><img src="https://github.com/imtiredwhenitalk/Elyra/actions/workflows/rust.yml/badge.svg?branch=main" alt="Rust build" /></a>
  <a href="https://imtiredwhenitalk.github.io/Elyra/"><img src="https://img.shields.io/badge/website-live-4d7a61?style=flat-square" alt="Website" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-4d7a61?style=flat-square&logo=windows&logoColor=white" alt="Windows 10 and 11" />
  <img src="https://img.shields.io/badge/stack-Rust%20%2B%20Tauri-4d7a61?style=flat-square&logo=rust&logoColor=white" alt="Rust and Tauri" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-4d7a61?style=flat-square" alt="MIT license" /></a>
</p>

<p>
  <a href="#download">Download</a> ·
  <a href="#features">Features</a> ·
  <a href="#security-built-in">Security</a> ·
  <a href="#build-from-source">Build from source</a> ·
  <a href="#documentation">Documentation</a>
</p>

</div>

---

## Why Elyra

- **Local by default.** No account is required to open a workspace. Mail, settings, and scan results live in a local SQLite database on your device.
- **Rust all the way down.** Mailbox queries, IMAP sync, sending, crypto, reputation, and attachment analysis run in the Rust core behind Tauri commands.
- **Security you can see.** Every verdict comes with a reason, from sender reputation to attachment risk, and external checks stay opt-in.
- **No mystery servers.** The webview is a presentation layer. It never receives your passwords, API keys, or cloud session tokens.

## Features

| Mail | Protection | Workspace |
| :--- | :--- | :--- |
| 📥 **Calm inbox**<br />Inbox, Important, Starred, Snoozed, Spam, Trash, Archive, Spaces, bulk actions, and a command palette. | 🛡️ **Elyra Shield**<br />Local phishing, social-engineering, sender-spoofing, and link checks with Safe / Suspicious / Malicious results. | 🪟 **Windows delivery**<br />MSI, per-user NSIS setup, a portable launcher, and SHA-256 verified installers. |
| ⭐ **Priority**<br />Important flags, Low → Urgent levels, automatic local suggestions, and priority filters. | 🚫 **Block & allow**<br />Address and domain rules, allowlist precedence, and automatic Spam or Trash routing. | ⚙️ **Rust core**<br />Mailbox, providers, crypto, diagnostics, and scanning behind typed Tauri commands. |
| ✍️ **Compose & sync**<br />IMAP inbox sync, SMTP sending, drafts, attachments, and a server relay fallback. | ✅ **Reputation**<br />SPF / DKIM / DMARC evaluation, domain classifications, reasons, and false-positive reports. | 🎨 **Themes**<br />Light, Dark, Midnight, OLED, four accents, density options, and reduced motion. |
| 📎 **Attachments**<br />Real-byte type detection, risk cards, scan history, and guarded downloads. | 🔐 **Keys & crypto**<br />AES-256-GCM, Ed25519 identity keys, OS keychain secrets, and BYOK integrations. | 🌐 **Open docs**<br />User, architecture, security, API, deployment, and troubleshooting guides in one place. |

## Security built in

Elyra checks things locally first. External intelligence is bring-your-own-key and never runs silently.

| Layer | What it does |
| :--- | :--- |
| 🛡️ **Elyra Shield** | Scores messages with local phishing and social-engineering heuristics, sender and display-name spoof checks, and visible-link mismatches. |
| ✅ **Sender reputation** | Classifies sender domains as Trusted, Neutral, Suspicious, or Malicious using allow/block rules, brand checks, SPF / DKIM / DMARC, and cached threat intelligence. |
| 🚫 **Sender rules** | Blocks or allows addresses and whole domains. Allow rules always win, and blocked mail can go to Spam or Trash. |
| 📎 **Attachment scanning** | Detects real file types from magic bytes, flags executables, scripts, disk images, macro documents, archives, double extensions, and hidden bidirectional text. |
| 🔗 **Threat intelligence** | Optional VirusTotal and Google Safe Browsing lookups with your own API keys, local caching, and throttling. |
| 🔐 **Crypto & secrets** | AES-256-GCM encryption, Ed25519 identity keys, and cloud/IMAP/SMTP secrets stored in the Windows Credential Manager. |

> **Consent first.** Attachments are never uploaded to VirusTotal unless you confirm each file. Files are never executed, opened, or auto-run by Elyra. ClamAV is opt-in and runs on this device.

## Download

The newest published installer is on the [releases page](https://github.com/imtiredwhenitalk/Elyra/releases/latest):

| Asset | Purpose |
| :--- | :--- |
| `Elyra-latest-x64.msi` | Standard Windows Installer package (per-machine, may request administrator approval) |
| `Elyra-latest-x64-setup.exe` | NSIS setup with a per-user install mode |
| `Elyra-Launcher-x64.exe` | Portable launcher that downloads, verifies, installs, and starts Elyra |
| `Elyra-latest-x64.msi.sha256` | SHA-256 checksum used by the launcher |

The launcher accepts `--msi <path>` for an already-downloaded installer and automatically fetches `<url>.sha256` when installing from a URL.

## Build from source

Requirements: Windows 10/11, Node.js 20+, Rust stable, and the platform prerequisites for Tauri 2.

```powershell
npm.cmd install --prefix apps/desktop
npm.cmd run check
npm.cmd run build        # MSI + NSIS bundles
cargo check -p tauri-app
cargo test -p elyra-crypto -p elyra-server -p elyra-launcher
```

The frontend is static (`apps/desktop/src`), so there is no bundler step. The browser preview fallback in `apps/desktop/src/services/tauri.js` keeps the UI usable without Tauri, while desktop builds always use the Rust commands.

Run the API server with `cargo run -p elyra-server` after starting PostgreSQL and copying [`server/.env.example`](server/.env.example).

## Architecture

```text
apps/desktop            Tauri 2 window + static HTML/CSS/JS
        │
        │ Tauri commands
        ▼
Rust core               mailbox · providers · shield · priority
                        senders · reputation · attachments · crypto
        │
        ├── SQLite      local mailbox, settings, scan results
        └── Keychain    cloud session, IMAP/SMTP passwords, API keys
```

The Rust workspace also contains the reusable [`elyra-crypto`](packages/crypto) package, the authenticated [`server`](server) service, and the Windows [`launcher`](apps/launcher). The server uses PostgreSQL for account metadata and sync cursors, Argon2id for password hashing, and signed access tokens. It never receives local encryption keys.

## Project layout

| Path | What it is |
| :--- | :--- |
| [`apps/desktop`](apps/desktop) | Tauri desktop app, static UI, and Rust commands |
| [`apps/launcher`](apps/launcher) | Portable Windows installer and updater CLI |
| [`packages/crypto`](packages/crypto) | Authenticated encryption primitives |
| [`server`](server) | Axum API, PostgreSQL migrations, auth, mail relay, and sync cursors |
| [`web`](web) | Static marketing and download site |
| [`infrastructure`](infrastructure) | Docker and deployment assets |
| [`docs`](docs/README.md) | User, developer, security, and deployment documentation |

## Documentation

| Page | Covers |
| :--- | :--- |
| [Getting started](docs/getting-started.md) | Install, launcher flags, onboarding, local data, uninstall |
| [User guide](docs/user-guide.md) | Mailboxes, reading, composing, mail accounts, settings |
| [Architecture](ai.md) | System diagram, project rules, stack, and repository layout |
| [Server and API](docs/server.md) | Local server, endpoints, PostgreSQL schema, SMTP relay |
| [Security](docs/security.md) | Keychain, crypto, server trust boundary, threat model, gaps |
| [Development](docs/development.md) | Prerequisites, commands, validation, release process |
| [Deployment](docs/deployment.md) | Production stack, HTTPS, backups, auto-deploy |
| [Configuration](docs/reference/configuration.md) | Every desktop, server, and deploy variable |
| [Tauri commands](docs/reference/tauri-commands.md) | Every command the webview can invoke |
| [Troubleshooting](docs/troubleshooting.md) | Install, sign-in, IMAP, sending, server, build issues |

The complete VPS walkthrough remains in [`docs/deploy/SERVER.md`](docs/deploy/SERVER.md).

## Local data

The desktop database lives at `%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3` on Windows. The exact path is shown in **Settings → Diagnostics**. SQLite uses WAL journaling, foreign keys, and secure deletion.

Cloud sessions, IMAP/SMTP passwords, and integration API keys live in the OS keychain, not in the database.

## Status & roadmap

The latest published installer is **0.3.0**. Phase 1 (Shield, threat intelligence, identity keys, BYOK integrations) and Phase 2 (priority, sender rules, reputation, attachment scanning) are complete on `main` and will ship in the next release.

Next up: Gmail/Outlook OAuth with PKCE, scheduled background sync, WebSocket live updates, and mailbox encryption at rest.

Track work in [`task.md`](task.md) and [`changes.md`](changes.md).

## Contributing

Read [`docs/development.md`](docs/development.md) for prerequisites, commands, and repository conventions. When you change a command, route, environment variable, schema, or deployment step, update the relevant doc page, add an entry to [`changes.md`](changes.md), and update [`task.md`](task.md) when the roadmap changes.

## License

Elyra is released under the [MIT License](LICENSE).

Copyright © 2026 Elyra contributors. Third-party dependencies keep their own licenses; see each crate or package for details.

---

<div align="center">

<p><strong>Built with Rust and Tauri. No telemetry. No mystery servers.</strong></p>

<p>
  <a href="https://github.com/imtiredwhenitalk/Elyra">GitHub</a> ·
  <a href="https://imtiredwhenitalk.github.io/Elyra/">Website</a> ·
  <a href="https://github.com/imtiredwhenitalk/Elyra/releases">Releases</a> ·
  <a href="docs/README.md">Docs</a> ·
  <a href="LICENSE">License</a>
</p>

</div>
