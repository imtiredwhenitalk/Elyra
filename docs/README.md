# Elyra documentation

This is the central documentation set for Elyra **0.4.0**. It describes what the
project actually ships today: a Windows desktop client built with Tauri 2 and
Rust, a local SQLite mailbox, a Rust/PostgreSQL API server, an optional SMTP
relay, a portable launcher, and a static public website.

Start with the page that matches what you want to do.

## I want to…

| Goal | Read |
| --- | --- |
| Install Elyra on Windows | [Getting started](getting-started.md) |
| Install Elyra on Linux | [Linux](linux.md) |
| Learn the interface and features | [User guide](user-guide.md) |
| Understand how the system fits together | [Architecture](architecture.md) |
| Run or extend the Rust server | [Server and API](server.md) |
| Deploy the website and API to a VPS | [Deployment](deployment.md) and [server runbook](deploy/SERVER.md) |
| Verify a production deployment | [Production verification checklist](deploy/VERIFY.md) |
| Review the security model | [Security](security.md) |
| Build from source, test, or release | [Development](development.md) |
| Run or extend the automated tests | [Development → automated tests](development.md#automated-tests) |
| Look up environment variables | [Configuration reference](reference/configuration.md) |
| Look up Tauri commands | [Tauri command reference](reference/tauri-commands.md) |
| Fix a problem | [Troubleshooting and FAQ](troubleshooting.md) |
| See what changed or what is next | [task.md](../task.md) and [changes.md](../changes.md) |

## Documentation map

### For users

- **[Getting started](getting-started.md)** — download options, launcher flags,
  first-run onboarding, local data, uninstall.
- **[Linux](linux.md)** — AppImage, .deb, Arch PKGBUILD, dependencies, source
  build, Flatpak status.
- **[Desktop updates](desktop-updates.md)** — signed Windows updater, Settings
  controls, signing keys, and the release checklist.
- **[User guide](user-guide.md)** — mailboxes, search, message viewer, compose,
  mail accounts, Elyra Cloud, settings, themes, keyboard shortcuts.
- **[Troubleshooting and FAQ](troubleshooting.md)** — install, sign-in, IMAP,
  sending, sync, server, and build issues.

### For developers

- **[Architecture](architecture.md)** — components, trust boundaries, data
  flows, directory layout, technology choices, and extension points.
- **[Development](development.md)** — prerequisites, commands, automated tests,
  validation, release process, repository conventions.
- **[Tauri command reference](reference/tauri-commands.md)** — every command the
  desktop webview can invoke.
- **[Configuration reference](reference/configuration.md)** — desktop, server,
  deployment, and build-time variables in one place.

### For operators

- **[Server and API](server.md)** — local server setup, data model, endpoint
  reference, SMTP relay, operational behavior.
- **[Deployment](deployment.md)** — production stack overview, quick start, and
  links to the full runbook.
- **[Server runbook](deploy/SERVER.md)** — the complete VPS walkthrough: DNS,
  secrets, Caddy HTTPS, backups, updates, GitHub Actions.
- **[Production verification](deploy/VERIFY.md)** — deployment checklist,
  smoke tests, HTTPS checks, restart persistence, backup/restore drill.
- **[Security](security.md)** — threat model, keychain usage, crypto boundary,
  server trust boundary, known gaps.

## What this documentation is not

- It is not a promise of features that do not exist. Provider OAuth with PKCE,
  WebSocket live updates, scheduled background sync, and mailbox encryption at
  rest are **not** implemented in 0.4.0. They are listed as next milestones in
  [task.md](../task.md).
- It does not replace the per-directory `ARCHITECTURE.md` markers. Those files
  are short scope notes for contributors working inside a directory. This
  `docs/` set is the narrative documentation.

## Conventions

- Repository paths are written relative to the repository root, for example
  `apps/desktop/src-tauri/src/mailbox.rs`.
- Windows commands use PowerShell syntax (`npm.cmd`, `$env:NAME`). Linux server
  commands use POSIX shell syntax.
- “Desktop” means the Tauri application. “Webview” means the HTML/CSS/JS layer
  inside it. “Core” means the Rust code in `apps/desktop/src-tauri`.
- A statement marked **implemented** is backed by code in this repository.
  A statement marked **planned** is tracked in `task.md` and is not available.

## Documentation sources of truth

When documentation and code disagree, the code wins. Use these files to verify
behavior:

| Topic | Source of truth |
| --- | --- |
| Desktop commands and setup | `apps/desktop/src-tauri/src/lib.rs` |
| Local database schema | `apps/desktop/src-tauri/src/storage.rs`, `mailbox.rs`, `providers.rs` |
| Cloud client behavior | `apps/desktop/src-tauri/src/cloud.rs` |
| Sending and relay fallback | `apps/desktop/services/sendemail.rs`, `server/src/mailer.rs` |
| HTTP routes and errors | `server/src/api.rs` |
| Server schema | `server/src/database.rs` |
| Production services | `infrastructure/deployment/docker-compose.prod.yml` |
| Installer and bundle config | `apps/desktop/src-tauri/tauri.conf.json` |
| Release pipeline | `.github/workflows/release.yml`, `.github/workflows/rust.yml` |
| Linux release pipeline | `.github/workflows/linux-release.yml`, `packaging/linux/PKGBUILD` |

## Keeping the docs current

Documentation changes do not require architecture approval, but they must
describe the implementation as it is. When you change a command, route,
environment variable, schema, or deployment step:

1. Update the relevant page in `docs/`.
2. Add an entry to [`changes.md`](../changes.md) with the `[200 OK]` marker used
   by this project.
3. Update [`task.md`](../task.md) if the change completes or adds a roadmap item.
4. Re-check links and code blocks.

## Related top-level files

- [`README.md`](../README.md) — project overview, downloads, and quick commands.
- [`LICENSE`](../LICENSE) — MIT license covering the repository.
- [`ARCHITECTURE.md`](../ARCHITECTURE.md) — short pointer to the architecture.
- [`ai.md`](../ai.md) — project rules, intended stack, and layout.
- [`task.md`](../task.md) — feature checklist and status.
- [`changes.md`](../changes.md) — chronological implementation log.
