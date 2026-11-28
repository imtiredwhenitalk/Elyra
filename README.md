> 🌐 **English** · [Українська](README.ua.md)

<div align="center">

<img src="web/favicon.svg" width="84" height="84" alt="Elyra" />

<h1>Elyra</h1>

<p><strong>Your inbox, back on your side.</strong></p>

<p>
  A quiet, local-first email workspace for Windows and Linux.<br />
  Mailbox logic runs in Rust, your mail lives in local SQLite,<br />
  and installers and updates are verified before they change your system.
</p>

<p>
  <a href="https://github.com/imtiredwhenitalk/Elyra/releases/latest"><img src="https://img.shields.io/github/v/release/imtiredwhenitalk/Elyra?style=flat-square&color=4d7a61&label=release" alt="Latest release" /></a>
  <a href="https://github.com/imtiredwhenitalk/Elyra/actions/workflows/rust.yml"><img src="https://github.com/imtiredwhenitalk/Elyra/actions/workflows/rust.yml/badge.svg?branch=main" alt="Rust build" /></a>
  <a href="https://imtiredwhenitalk.github.io/Elyra/"><img src="https://img.shields.io/badge/website-live-4d7a61?style=flat-square" alt="Website" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-4d7a61?style=flat-square&logo=windows&logoColor=white" alt="Windows 10 and 11" />
  <img src="https://img.shields.io/badge/platform-Linux%20x86_64-4d7a61?style=flat-square&logo=linux&logoColor=white" alt="Linux x86_64" />
  <img src="https://img.shields.io/badge/stack-Rust%20%2B%20Tauri-4d7a61?style=flat-square&logo=rust&logoColor=white" alt="Rust and Tauri" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-4d7a61?style=flat-square" alt="MIT license" /></a>
</p>

<p>
  <a href="#download">Download</a> ·
  <a href="#features">Features</a> ·
  <a href="#updates-and-uninstall">Updates &amp; uninstall</a> ·
  <a href="#build-from-source">Build from source</a> ·
  <a href="#documentation">Documentation</a>
</p>

</div>

---

## Why Elyra

- **Local by default.** No account is required to open a workspace. Mail, settings, and scan results live in a local SQLite database on your device.
- **Rust all the way down.** Mailbox queries, IMAP sync, sending, crypto, reputation, and attachment analysis run in the Rust core behind Tauri commands.
- **Security you can see.** Every verdict comes with a reason, from sender reputation to attachment risk, and external checks stay opt-in.
- **Installers you can trust.** Downloads use stable asset names and SHA-256 sidecars; Windows updates are signature-verified before installation.
- **No mystery servers.** The webview is a presentation layer. It never receives your passwords, API keys, or cloud session tokens.
- **Your server, if you want one.** A production Compose stack, deployment script, verification checklist, and database integration tests support self-hosting.

## Features

| Mail | Protection | Workspace |
| :--- | :--- | :--- |
| 📥 **Calm inbox**<br />Inbox, Important, Starred, Snoozed, Spam, Trash, Archive, Spaces, bulk actions, and a command palette. | 🛡️ **Elyra Shield**<br />Local phishing, social-engineering, sender-spoofing, and link checks with Safe / Suspicious / Malicious results. | 🪟 **Windows delivery**<br />MSI, per-user NSIS setup, a signed-update path, and SHA-256 verified installers. |
| ⭐ **Priority**<br />Important flags, Low → Urgent levels, automatic local suggestions, and priority filters. | 🚫 **Block & allow**<br />Address and domain rules, allowlist precedence, and automatic Spam or Trash routing. | 🐧 **Linux packages**<br />AppImage and `.deb` builds, an Arch PKGBUILD, desktop entry, icon, and checksums. |
| ✍️ **Compose & sync**<br />IMAP inbox sync with app passwords or OAuth PKCE, SMTP sending, drafts, attachments, and a server relay fallback. | ✅ **Reputation**<br />SPF / DKIM / DMARC evaluation, domain classifications, reasons, and false-positive reports. | ⚙️ **Rust core**<br />Mailbox, providers, crypto, diagnostics, and scanning behind typed Tauri commands. |
| 📎 **Attachments**<br />Real-byte type detection, risk cards, scan history, and guarded downloads. | 🔐 **Keys & crypto**<br />AES-256-GCM, Ed25519 identity keys, OS keychain secrets, and BYOK integrations. | 🎨 **Themes**<br />Light, Dark, Midnight, OLED, four accents, density options, and reduced motion. |
| 🔄 **Verified updates**<br />Windows checks a signed manifest, verifies the installer, and reports progress. | 🚀 **Self-hosting**<br />Compose, optional Caddy HTTPS, health checks, and an opt-in deploy workflow. | 📚 **Open docs**<br />User, architecture, security, API, deployment, and troubleshooting guides in one place. |

## Security built in

Elyra checks things locally first. External intelligence is bring-your-own-key and never runs silently.

| Layer | What it does |
| :--- | :--- |
| 🛡️ **Elyra Shield** | Scores messages with local phishing and social-engineering heuristics, sender and display-name spoof checks, and visible-link mismatches. |
| ✅ **Sender reputation** | Classifies sender domains as Trusted, Neutral, Suspicious, or Malicious using allow/block rules, brand checks, SPF / DKIM / DMARC, and cached threat intelligence. |
| 🚫 **Sender rules** | Blocks or allows addresses and whole domains. Allow rules always win, and blocked mail can go to Spam or Trash. |
| 📎 **Attachment scanning** | Detects real file types from magic bytes, flags executables, scripts, disk images, macro documents, archives, double extensions, and hidden bidirectional text. |
| 🔗 **Threat intelligence** | Optional VirusTotal and Google Safe Browsing lookups with your own API keys, local caching, and throttling. |
| 🔐 **Crypto & secrets** | AES-256-GCM encryption, Ed25519 identity keys, and secrets stored in the OS keychain where available (Windows Credential Manager today; Linux Secret Service support is still hardening). |

> **Consent first.** Attachments are never uploaded to VirusTotal unless you confirm each file. Files are never executed, opened, or auto-run by Elyra. ClamAV is opt-in and runs on this device.

## Download

The newest published assets are on the [releases page](https://github.com/imtiredwhenitalk/Elyra/releases/latest) and on the [website](https://imtiredwhenitalk.github.io/Elyra/#download).

| Asset | Purpose |
| :--- | :--- |
| `Elyra-latest-x64.msi` | Standard Windows Installer package (per-machine, may request administrator approval) |
| `Elyra-latest-x64-setup.exe` | NSIS setup with a per-user install mode |
| `Elyra-Launcher-x64.exe` | Portable launcher that downloads, verifies, installs, updates, and can uninstall Elyra |
| `Elyra-latest-x64.msi.sha256` | SHA-256 checksum used by the launcher and manual verification |
| `Elyra-latest-x64.AppImage` | Portable Linux x86_64 AppImage |
| `Elyra-latest-x64.deb` | Debian/Ubuntu package |
| `Elyra-latest-x64.AppImage.sha256` | SHA-256 checksum used by the PKGBUILD and manual verification |
| `Elyra-latest-x64.deb.sha256` | SHA-256 checksum for the Debian package |

The portable launcher supports resumable downloads, HTTP Range retries, atomic `.part` files, checksum sidecars, a verified MSI cache, and install/launch/update modes. Linux packages are published to each tagged GitHub Release; see [Linux](docs/en/linux.md).

## Updates and uninstall

Windows builds include **Settings → Updates**: an on-demand check, current and latest version details, release notes, download and verification progress, install, and preferences for automatic checks and notifications. The updater public key is compiled into the app; a failed check, bad signature, or failed download leaves the installed version untouched. Signing secrets and the release checklist are documented in [Desktop updates](docs/en/desktop-updates.md).

Uninstalling is non-destructive by default. Use Windows **Settings → Apps → Elyra**, or preview the launcher cleanup first:

```powershell
.\Elyra-Launcher-x64.exe uninstall --dry-run
```

Add `--remove-data` (or `--purge`) to remove the local database and matching keychain entries, optionally with `--backup-data <folder>`. See [Getting started](docs/en/getting-started.md#9-uninstall) for every flag.

Linux installs are updated by replacing the checksummed AppImage or `.deb`; package removal does not delete local mail or settings.

## Build from source

### Windows

Requirements: Windows 10/11, Node.js 20+, Rust stable, and the Tauri 2 prerequisites (Microsoft C++ Build Tools with the **Desktop development with C++** workload).

```powershell
npm.cmd install --prefix apps/desktop
npm.cmd run check
npm.cmd run build        # MSI + NSIS bundles
cargo check -p tauri-app
cargo test -p elyra-crypto -p elyra-server -p elyra-launcher
```

### Linux

Requirements: a Linux distribution with Tauri 2 system libraries (WebKit2GTK 4.1, GTK 3, AppIndicator), Node.js 20+, and Rust stable.

```sh
npm install --prefix apps/desktop
npm run check
npm --prefix apps/desktop run tauri -- build --bundles appimage deb
cargo test -p elyra-crypto -p elyra-server -p elyra-launcher
```

The frontend is static (`apps/desktop/src`), so there is no bundler step. The browser preview fallback in `apps/desktop/src/services/tauri.js` keeps the UI usable without Tauri, while desktop builds always use the Rust commands.

By default, database-backed server tests skip when no PostgreSQL is available. To enforce them against a disposable database:

```powershell
$env:ELYRA_TEST_DATABASE_URL = "postgres://elyra:elyra@127.0.0.1:5432/elyra_test"
cargo test -p elyra-server
```

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
        ├── Keychain    cloud session, IMAP passwords, OAuth token sets, API keys
        └── Updater     signed manifest + verified Windows installers

apps/launcher           download → verify → install → launch / uninstall
server                  Axum API + PostgreSQL + SMTP relay (self-hosted)
web                     static marketing and download site
```

The Rust workspace also contains the reusable [`elyra-crypto`](packages/crypto) package, the authenticated [`server`](server) service, and the Windows [`launcher`](apps/launcher). The server uses PostgreSQL for account metadata and sync cursors, Argon2id for password hashing, and signed access tokens. It never receives local encryption keys.

Release automation lives in `.github/workflows`: `rust.yml` checks both platforms, `release.yml` builds and signs Windows assets with `latest.json`, `linux-release.yml` builds AppImage and `.deb`, and `deploy.yml` is an opt-in SSH deployment for the self-hosted API.

## Project layout

| Path | What it is |
| :--- | :--- |
| [`apps/desktop`](apps/desktop) | Tauri desktop app, static UI, Rust commands, and updater |
| [`apps/launcher`](apps/launcher) | Portable Windows downloader, installer, updater, and uninstall CLI |
| [`packages/crypto`](packages/crypto) | Authenticated encryption primitives |
| [`server`](server) | Axum API, PostgreSQL migrations, auth, mail relay, and sync cursors |
| [`web`](web) | Static marketing and download site |
| [`infrastructure`](infrastructure) | Docker and deployment assets |
| [`docs`](docs/en/README.md) | User, developer, security, deployment, and release documentation |

## Documentation

| Page | Covers |
| :--- | :--- |
| [Getting started](docs/en/getting-started.md) | Install, launcher flags, onboarding, local data, uninstall |
| [Linux](docs/en/linux.md) | AppImage, .deb, Arch PKGBUILD, dependencies, Flatpak status |
| [Desktop updates](docs/en/desktop-updates.md) | Signed Windows updates, Settings controls, signing keys, release checklist |
| [User guide](docs/en/user-guide.md) | Mailboxes, reading, composing, mail accounts, settings |
| [Provider OAuth](docs/en/oauth.md) | Gmail/Outlook Authorization Code + PKCE, keychain tokens, IMAP XOAUTH2 limits |
| [Architecture](ai.md) | System diagram, project rules, stack, and repository layout |
| [Server and API](docs/en/server.md) | Local server, endpoints, PostgreSQL schema, SMTP relay |
| [Security](docs/en/security.md) | Keychain, crypto, server trust boundary, threat model, gaps |
| [Development](docs/en/development.md) | Prerequisites, commands, validation, release process |
| [Deployment](docs/en/deployment.md) | Production stack, HTTPS, backups, auto-deploy |
| [Production verification](docs/en/deploy/VERIFY.md) | Pre-deploy checks, HTTPS, restart persistence, backup/restore drill |
| [Configuration](docs/en/reference/configuration.md) | Every desktop, server, and deploy variable |
| [Tauri commands](docs/en/reference/tauri-commands.md) | Every command the webview can invoke |
| [Troubleshooting](docs/en/troubleshooting.md) | Install, sign-in, IMAP, sending, server, build issues |

The complete VPS walkthrough remains in [`docs/en/deploy/SERVER.md`](docs/en/deploy/SERVER.md).

## Local data

On Windows the desktop database lives at `%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3`. On Linux it lives under the XDG data directory, typically `~/.local/share/com.elyra.desktop/elyra.sqlite3`. **Settings → Diagnostics** shows the exact path. SQLite uses WAL journaling, foreign keys, and secure deletion.

Cloud sessions, IMAP passwords, provider OAuth token sets, and integration API keys live in the OS keychain, not in the database. Uninstalling the app does not delete them or the mailbox unless you explicitly ask the launcher to do so.

## Status & roadmap

Version **0.4.0** is the current release line. Phase 1 (Shield, threat intelligence, identity keys, BYOK integrations), Phase 2 (priority, sender rules, reputation, attachment scanning), Phase 3 (launcher/uninstall, Linux packages, server testing and production verification, signed Windows updates), and Phase 4 (redacted local diagnostics, advanced search, offline outbox, accessibility polish) are implemented on `main`; the Rust CI checks both Windows and Linux.

In 0.4.0, errors are logged locally after redaction, advanced filters run entirely in SQLite, failed sends wait in a durable outbox, and destructive actions use keyboard-accessible confirmations. No diagnostic data is uploaded automatically.

Gmail/Outlook OAuth with PKCE is available for desktop IMAP sync. Next up: SMTP XOAUTH2 for those accounts, scheduled background sync, WebSocket live updates, and mailbox encryption at rest. Flatpak packaging was investigated and documented, but is not shipped in 0.4.0.

Track work in [`task.md`](task.md), [`changes.md`](changes.md), and the local [`issues/`](issues/README.md) files; GitHub issue templates are in [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/).

## Contributing

Read [`docs/en/development.md`](docs/en/development.md) for prerequisites, commands, and repository conventions. When you change a command, route, environment variable, schema, or deployment step, update the relevant doc page, add an entry to [`changes.md`](changes.md), and update [`task.md`](task.md) when the roadmap changes.

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
  <a href="docs/en/README.md">Docs</a> ·
  <a href="LICENSE">License</a>
</p>

</div>
