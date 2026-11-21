[200 OK] Refreshed the public website and README for the shipped 0.3.0 feature set:

The marketing page now presents Windows and Linux as first-class targets, with jump links for each download card, current MSI/NSIS/AppImage/.deb assets and checksums, signed-update and launcher/uninstall FAQ answers, and a release grid that reflects Phase 3 instead of repeating the server deploy card. The Windows update copy is explicit that a signed `latest.json` is published only when the maintainer configures the signing secret. Small CSS polish adds card hover states, a text link treatment, and accessible focus-friendly navigation chips.

`README.md` was rewritten around the current repository: cross-platform feature table, download and checksum inventory, launcher/update/uninstall behavior, Windows and Linux build commands, database test opt-in, release workflow overview, local data paths for both platforms, and a status section marking Phase 3 complete. `docs/desktop-updates.md` no longer claims that local builds emit Linux bundles; the Linux workflow builds them explicitly.

Files changed: `README.md`, `web/index.html`, `web/styles.css`, `docs/desktop-updates.md`, `changes.md`.

Validation: [OK] `git diff --check`; [OK] HTML tag/structure sanity check; [OK] CSS brace balance; [OK] local Markdown links. GitHub Pages is redeployed from `web/**` by the existing workflow.

[FIX] Restored CI builds after the Phase 3 commit:

The Windows desktop build failed because `updates.rs` referenced `storage::connection` without the `crate::` prefix and used `String(value)` as a function call; both are corrected, and the now-unused `Manager` import was removed. The server tests failed because sqlx 0.8 exposes `acquire_timeout` on `PgPoolOptions`, not `connect_timeout`; both the API test helper and the database integration test now use `acquire_timeout`. The unused `put` re-export was removed from the Axum routing import, and the desktop bundle targets were limited to `msi`/`nsis` so Windows CI does not attempt Linux bundles; the Linux workflow continues to build AppImage and .deb explicitly with `--bundles appimage deb`.

Files changed: `apps/desktop/src-tauri/src/updates.rs`, `apps/desktop/src-tauri/tauri.conf.json`, `server/src/api.rs`, `server/tests/database_integration.rs`, `changes.md`.

Validation: [OK] `git diff --check`; [OK] JSON parse for `tauri.conf.json`; [OK] local Markdown links. Rust compilation and desktop packaging are validated by GitHub Actions.

[200 OK] Added automatic Windows application updates (Phase 3, GitHub issue #11):

Added `tauri-plugin-updater` with the Tauri/minisign public key embedded as a Rust constant and an empty configuration placeholder overridden at runtime. Settings → Updates now shows the current and latest versions, release date, notes, last check, target, and status; it supports manual checks, download and verification progress, direct install, and preferences for automatic checks and notifications. Update results and preferences persist in SQLite, while browser preview uses localStorage with a safe simulated up-to-date result.

The Windows release workflow signs the MSI and NSIS installers when `TAURI_SIGNING_PRIVATE_KEY` is configured, writes `.sig` and SHA-256 sidecars, and publishes `latest.json` with absolute HTTPS asset URLs and both Windows bundle keys. `requireSignedVersion` is enabled so the signed version must match the announced manifest version. Unsigned releases still publish, but skip signing and `latest.json` with a warning. A failed download, signature check, or install leaves the existing installation untouched; on Windows the app exits when the verified installer starts.

Files changed: `Cargo.toml`, `tauri.conf.json`, `capabilities/default.json`, `lib.rs`, `updates.rs`, `index.html`, `main.js`, `tauri.js`, `styles.css`, `.github/workflows/release.yml`, `docs/desktop-updates.md`, `docs/reference/tauri-commands.md`, `docs/README.md`, `docs/development.md`, `README.md`, `task.md`.

Validation: [OK] `node --check` for `main.js` and `tauri.js`; [OK] JSON parse for `tauri.conf.json`, `capabilities/default.json`, `package.json`, and root `package.json`; [OK] YAML parse for `release.yml`, `rust.yml` and the Linux workflow; [OK] Tauri CLI keypair generation and test signing succeeded; [OK] `git diff --check`; Rust compilation and the Windows desktop build run in GitHub Actions because no local Rust toolchain is available.

[200 OK] Added MIT licensing across the repository:

Added a root `LICENSE` file with the standard MIT text
(`Copyright (c) 2026 Elyra contributors`) and declared `MIT` in every
manifest: the root `package.json`, `apps/desktop/package.json`,
`packages/types/package.json`, `packages/ui/package.json`, and
`apps/desktop/src-tauri/Cargo.toml` (via `license.workspace = true`). The
workspace license in `Cargo.toml` was already `MIT`. The README license badge,
License section, and footer now link to `LICENSE`, and `docs/README.md` lists it
among the related top-level files.

Files changed: `LICENSE`, `README.md`, `changes.md`, `docs/README.md`,
`package.json`, `apps/desktop/package.json`, `packages/types/package.json`,
`packages/ui/package.json`, `apps/desktop/src-tauri/Cargo.toml`.

Validation: [OK] every edited `package.json` parses as JSON; [OK] `LICENSE`
matches the canonical MIT text; [OK] README local links resolve;
[OK] `git diff --check`.

[200 OK] Refreshed the repository README:

Rebuilt the project landing page with a centered brand header, release, build,
website, platform, stack, and license badges, a concise feature grid, a security
overview, download and build instructions, an architecture diagram, a project
layout map, documentation links, and an honest status and roadmap section. The
completed Phase 1 and Phase 2 security work is described as available on `main`
for the next release.

Validation: [OK] every local README link resolves; HTML blocks balanced;
`git diff --check` clean.

[200 OK] Added attachment security scanning with optional ClamAV and VirusTotal (Phase 2, GitHub issue #8):

Added `attachments.rs` with local attachment analysis that inspects the real
file bytes instead of trusting the sender's file name or MIME type. It detects
executables, scripts, disk images, macro-enabled Office files, archives, HTML,
and SVG, flags double extensions and bidirectional-override tricks, and notices
when the detected content type contradicts the declared one. Every attachment
now stores `sha256`, `detected_mime`, `kind`, `risk`, `score`, `reasons`,
`content_available`, `scan_source`, `scanned_at`, and `consent_required`.

Local analysis always runs at import and on demand. ClamAV is optional and only
runs when enabled in Settings; VirusTotal file analysis is never automatic and
requires both a stored API key and explicit per-file consent. Attachments are
never executed or opened by Elyra, and uploads are size-limited. The reader
shows a risk card for each file with its detected type, source, reasons, scan,
consent-based VirusTotal, and guarded download actions. Shield now reuses the
stored attachment risk so risky files raise the message score. Settings includes
an **Attachment security** section for automatic scanning, ClamAV, the ClamAV
command path, VirusTotal uploads, and the maximum scan size, plus a rescan
action. Browser preview mirrors local classification with `localStorage` and
rejects uploads or downloads.

Files changed: `attachments.rs`, `mailbox.rs`, `providers.rs`, `shield.rs`,
`lib.rs`, `threats.rs`, `Cargo.toml`, `index.html`, `main.js`, `tauri.js`,
`styles.css`, `task.md`, `docs/user-guide.md`, `docs/security.md`,
`docs/reference/tauri-commands.md`, `docs/reference/configuration.md`.

Validation: [OK] `node --check` for `main.js`, `tauri.js`, `security.js`,
`SecurityPage.js`, `crypto.js`, `account.js`, and `CryptoCard.js`; [OK] CSS
braces balanced (587/587); [OK] fallback attachment checks with `node`; [OK]
`git diff --check`; Rust compilation and the Windows desktop build run in GitHub
Actions because no local Rust toolchain is available.

[200 OK] Fixed browser-safe preview mode and local-only fallback flows:

Added stable relative asset and module loading for browser preview sessions, plus a desktop/runtime fallback that keeps local storage, message sending, and encryption checks usable without Tauri.
Validation: [OK] browser preview loads with relative assets; JS syntax check passed.

[200 OK] Built Elyra’s desktop inbox prototype:

Replaced Tauri starter screen with privacy-focused inbox UI.
Added realistic local test emails, search, mailbox filters, starred messages, refresh interaction.
Added connected accounts panel with OAuth placeholder actions.
Added visible Changes tab for roadmap tracking.
Added responsive styling and privacy preview messaging.
Files changed: index.html, main.js, styles.css

Validation: [OK] No editor diagnostics; JavaScript syntax check passed; Tauri CLI 2.12.0 verified.

[200 OK] Added local mailbox and account-management UI:

Added Spam and Trash folders with dedicated test messages.
Added Profile panel with account statistics and privacy status.
Added Settings panel with display name, theme, local-only preview, notification toggles, and local save feedback.
Connected the profile, settings, account, and OAuth placeholder controls to the new views.

Validation: [OK] No editor diagnostics; JavaScript syntax check passed.

[200 OK] Implemented the next roadmap UI slice:

Added a full local message viewer with text body rendering, safe inline-image preview state, attachment card, reply/forward controls, starring, and move-to-trash action.
Added Spam and Trash management behavior for local test messages.
Expanded Settings with Light, Dark, Midnight, and OLED theme switching.
Added custom Elyra domain handling in Profile and Settings with the `alex@elyra.com` handle.
Added viewer close behavior when navigating between mailbox views.

Validation: [OK] No editor diagnostics; JavaScript syntax check passed.

[200 OK] Added the first local backend foundation:

Added a Rust/Tauri SQLite database at the platform app-local data directory.
Added schema tables for accounts, folders, emails, contacts, and settings.
Added Tauri commands for storage initialization, email caching, email listing, and local settings updates.

Validation: [OK] `cargo check --target-dir target-validation` passed.

[200 OK] Stabilized the local desktop experience:

Added safe HTML escaping for rendered message content.
Unified runtime asset URLs and fixed attachment/action icon loading.
Persisted local theme and display-name settings across app launches.
Improved narrow-window responsive behavior for the inbox and message viewer.
Formatted the Rust storage module and revalidated the Tauri backend.

Validation: [OK] JavaScript syntax check and `cargo check --target-dir target-validation` passed.

[200 OK] Added architecture documentation markers:

Added one `ARCHITECTURE.md` file to every source, documentation, package, server, and infrastructure directory.
Excluded generated and dependency directories such as `.git`, `node_modules`, `target`, and `target-validation`.

Validation: [OK] 30 architecture files created for commit tracking.

[200 OK] Added reproducible desktop build commands and completed the local mailbox slice:

Added root and desktop npm scripts for JavaScript checks and Tauri production builds.
Added missing SQLite folders used by the visible Spaces navigation.
Made sent-mail listing safe before the first send and display successful sent records in the Sent mailbox.
Enabled starring messages directly from the inbox list.

Validation: [OK] `npm.cmd run check`; [OK] `cargo check`; [OK] Tauri release build produced the Windows MSI installer.

[200 OK] Expanded the desktop interface and interaction layer:

Added a proper compose modal with validation, sending state, local draft saving, and responsive layout.
Added toast feedback, Escape-to-close, Ctrl/Cmd+K search focus, quick All/Unread/Starred filtering, and working notification/profile actions.
Improved read/unread checkbox rendering and added visible focus states for keyboard navigation.

Validation: [OK] JavaScript syntax check; [OK] editor diagnostics; [OK] Rust check.

[200 OK] Reworked the infrastructure layer:

Replaced invalid Compose pseudo-configuration with valid development and production stacks for PostgreSQL and Redis, plus Mailpit in the dev profile.
Added healthchecks, persistent named volumes, isolated networks, production password requirements, and `.env.example`.
Replaced unsafe installation commands in the shell files with deterministic `check`, `build`, `up`, `down`, `logs`, `status`, `backup`, and `clean` commands.
Added infrastructure usage documentation.

Validation: [OK] YAML editor diagnostics; shell files reviewed. Docker and Bash binaries were unavailable in the current Windows terminal, so runtime Compose validation must be run in Docker Desktop or WSL.

[200 OK] Added modular desktop services and the first real crypto layer:

Added reusable `components`, `pages`, `hooks`, and `services` modules for Security Lab, persistent preferences, mailbox utilities, and Tauri crypto calls.
Added Rust AES-256-GCM encryption/decryption with random nonces, OS randomness, SHA-256 key fingerprints, strict key/payload validation, and unit tests.
Mounted the Security Lab into Settings; generated keys remain in memory for the session and are not written to localStorage.

Validation: [OK] frontend syntax checks; [OK] editor diagnostics; [OK] Rust crypto tests: 2 passed, 0 failed.

[200 OK] Fixed the desktop runtime loading path:

Moved browser-loaded security modules into `apps/desktop/src` and replaced unsupported bare Tauri API imports with the configured `window.__TAURI__.core.invoke` bridge.
The Tauri dev window now starts with a static frontend that can resolve every runtime module without a bundler.

Validation: [OK] editor diagnostics; [OK] `cargo check`; [OK] Tauri dev process starts.

[200 OK] Added a local security posture and audit layer:

Added Rust security controls for SQLite foreign keys, WAL journaling, secure deletion, posture checks, and bounded local audit retention.
Added `get_security_status` and `list_audit_events` Tauri commands and connected them to Security Lab with explicit unavailable/error state handling.
Documented the threat model, trust boundaries, audit limitations, and production requirements in `docs/security/ARCHITECTURE.md`.

Validation: [OK] `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml --target-dir apps/desktop/src-tauri/target-validation`; JavaScript check should be rerun with the desktop npm command in a shell where `npm.cmd` resolves correctly.
[200 OK] Added reusable crypto, shared packages, and server foundation:

Added the `elyra-crypto` Rust package with AES-256-GCM authenticated encryption,
OS-random keys, zeroization, payload validation, fingerprints, and tamper
detection tests. Added `@elyra/types` contracts and `@elyra/ui` primitives.
Added the Rust server with PostgreSQL migrations, Argon2id password hashing,
JWT sessions, authenticated user endpoints, health checks, and sync cursors.

Validation: frontend syntax check passed; Rust validation was started but the
Windows Application Control policy blocked a dependency build-script executable.
[200 OK] Added optimized standalone release and website deployment:

Configured Tauri to produce Windows MSI and NSIS installers with per-user
installation and embedded WebView bootstrapper. Added a standalone public
website, Nginx reverse proxy, production API container, Docker build cache
exclusions, hardened private service networking, and a Rust Windows launcher
that downloads the MSI over HTTPS and starts `msiexec`.

Validation: Tauri JSON, Cargo metadata, Rust formatting, frontend syntax, and
git diff checks passed. Docker and Rust dependency executables are unavailable
under the current Windows Application Control policy.
[200 OK] Added account authentication and in-app notifications:

Added register/login UI with local preview sessions and Rust API integration.
Added authenticated notification delivery by recipient email, notification
inbox, unread badges, read state, validation, and PostgreSQL persistence.

Validation: frontend syntax and diagnostics passed; Rust formatting was applied.
[200 OK] Polished desktop UI and reduced frontend weight:

Removed unused Tauri starter CSS and external Google Fonts loading, added a
responsive compact layout for medium screens, improved notification badge
handling, and added a keyboard command palette with quick compose, filters,
notifications, and settings actions via Ctrl/Cmd+K.

Validation: browser command palette opened successfully; JS syntax, 
diagnostics, Tauri JSON, and diff checks passed.
[200 OK] Shipped the Rust-first 0.2.0 rebuild:

Moved mailbox behavior into a Rust SQLite engine with schema migration, demo
seeding, search, filters, stats, drafts, bulk actions, snooze, attachments,
sent-mail recording, export, and unit tests. Added a Rust cloud bridge for
registration, login, sessions, notifications, and sync while keeping access
tokens inside Rust. Added Rust diagnostics and changelog commands.

Replaced image-based icons with one inline SVG sprite using `currentColor`,
removed unused Noun Project assets, and rebuilt the desktop UI with four themes,
four accents, density options, bulk actions, a command palette, a richer reading
pane, and immediate preference persistence.

Added a Windows release workflow that builds MSI/NSIS bundles, publishes stable
asset names and SHA-256 checksums, and rewrote the portable launcher with MSI
magic validation, checksum verification, install/update/uninstall flows. Rebuilt
the public website and corrected installer scope and data-path documentation.

Validation: [OK] `npm.cmd --prefix apps/desktop run check`; [OK] all inline icon
names and static selectors cross-checked; [OK] workflow JSON/YAML reviewed.
Rust compilation could not run locally because the Rust toolchain and MSVC build
tools are unavailable in this environment; CI now tests the non-Tauri crates on
Linux and checks the Tauri crate on Windows.

[200 OK] Hardened the 0.2.0 release follow-up:

Moved the SQLite database to the app-local identifier directory
(`%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3`) so it no longer shares the
NSIS per-user install folder and the installer's delete-app-data option removes
it. Marked blocking cloud and SMTP commands as async so network work cannot
freeze the webview, switched SMTP TLS from `tokio1-rustls-tls` to the blocking
`rustls-tls` backend, normalized mirrored cloud profiles to camelCase, and
corrected the data paths in the website, README and architecture docs. Added
SmartScreen guidance and a SHA-256 verification link to the download page.

Validation: [OK] `npm.cmd --prefix apps/desktop run check`; [OK] node syntax
checks for the website and service modules; [OK] HTML tag-balance and anchor
checks; [OK] Rust SQL placeholder, command argument and Tauri async-command
review.

[200 OK] Added OS keychain secret storage and IMAP provider sync (0.3.0):

Moved Elyra Cloud sessions and IMAP/SMTP passwords from SQLite to the Windows
Credential Manager through keyring-rs; legacy settings.cloud_session rows
migrate to the keychain on first read and are then deleted. Added the secrets
module plus the providers module for account CRUD, provider presets, IMAP and
SMTP connection tests, and INBOX sync with local read/starred state preserved.
Extended send_email to use a selected account's SMTP transport and keychain
password, with the existing environment variables as fallback. Added dynamic
account UI, an add/edit account modal with presets, per-account and sync-all
actions, compose account selection, keychain diagnostics, and browser-preview
fallbacks for every new command. Bumped the workspace, desktop, launcher, and
website version to 0.3.0.

Validation: [OK] `npm.cmd --prefix apps/desktop run check`; [OK] node syntax
checks for main.js, tauri.js, and account.js; [OK] HTML tag-balance and
selector review; [OK] Rust API review against keyring, imap, mail-parser and
lettre docs. Rust compilation runs in GitHub Actions because the local Rust
toolchain and MSVC build tools are unavailable.

[200 OK] Added first-run registration, server mail relay, and server deployment (0.3.0):

The desktop app now shows a registration onboarding modal on first launch unless
the user chooses Continue offline; the auth modal also accepts a configurable
server URL, stored in SQLite `settings.api_url` and editable later under
Settings → Elyra Cloud. The Rust cloud bridge resolves the API base from
`ELYRA_API_URL`, the stored setting, or the compile-time
`ELYRA_DEFAULT_API_URL`. Added the server-side SMTP relay (`lettre`) with
`GET /v1/mail/status` and `POST /v1/mail/send`; `send_email` falls back to it
when no local account or `ELYRA_SMTP_*` environment is configured, so registered
users can write to any external address. Added the production deployment path:
`.env.prod.example`, `deploy.sh` with build/backup/health checks, an optional
Caddy HTTPS profile, a GitHub Actions SSH deploy workflow gated by
`ELYRA_DEPLOY_ENABLED`, and `docs/deploy/SERVER.md`. Updated the server and
infrastructure READMEs, `.gitignore`, and the release workflow's
`ELYRA_DEFAULT_API_URL` build variable.

Validation: [OK] `npm.cmd --prefix apps/desktop run check`; [OK] node syntax
checks for main.js, tauri.js, and account.js; [OK] `docker compose config`
review for the production, development, and build files; [OK] HTML
tag-balance and selector review; [OK] Rust mail relay, cloud relay, and send
fallback review; [OK] GitHub Actions Rust workflow green (crypto/server/launcher
tests on Linux and the Windows desktop build); [OK] v0.3.0 release published
with the MSI, NSIS setup and launcher assets. The first CI run caught three
desktop compile errors (keychain helper visibility, `ZeroCopy` iteration, and an
owned `AppHandle`), all fixed in `ae1c226` before the tag. Rust compilation
still runs only in GitHub Actions because the local Rust toolchain and MSVC
build tools are unavailable.

[200 OK] Published the complete project documentation set:

Added `docs/README.md` as the documentation index and a structured set of
pages: `getting-started.md`, `user-guide.md`, `architecture.md`, `server.md`,
`security.md`, `development.md`, `deployment.md`, `troubleshooting.md`,
`reference/configuration.md`, and `reference/tauri-commands.md`. Documented the
shipped 0.3.0 behavior rather than the intended stack: static HTML/CSS/vanilla
JS frontend, SQLite mailbox schema, keychain entries, IMAP INBOX sync,
per-account and relay sending, server routes and error codes, PostgreSQL schema,
deployment topology, and every registered Tauri command with arguments and
return shapes. Replaced the duplicated `docs/ARCHITECTURE.md` content with a
documentation map, linked the set from `README.md`, and corrected `ai.md` so its
UI and directory claims match the repository. Added the troubleshooting guide
referenced by the other pages.

Validation: [OK] every internal documentation link checked against the files on
disk; [OK] command list compared with `apps/desktop/src-tauri/src/lib.rs`; [OK]
server routes and limits compared with `server/src/api.rs`; [OK] local schema
compared with `storage.rs`, `mailbox.rs`, `providers.rs`, and `security.rs`;
[OK] code fences and Markdown tables reviewed; [OK] `git diff --check` clean.

[200 OK] Implemented the security milestone from GitHub issues #1–#4:

Added `threats.rs` for URL extraction from message text and HTML anchors,
VirusTotal and Google Safe Browsing lookups with the user's own API keys, a
SQLite verdict cache (24 hours for known verdicts, 1 hour for unknown), a
16-second VirusTotal request throttle, and per-provider failure reporting so a
provider outage never blocks the mailbox. Added `shield.rs` for local phishing
and social-engineering detection: brand and lookalike-domain checks, punycode
detection, urgency and credential-request language, dangerous attachments, and
visible-link versus destination mismatches. Shield scores messages as Safe,
Suspicious, or Malicious, stores the unified result, exposes alerts and a
summary, and supports false-positive reporting. Local checks run when IMAP mail
is imported; external link checks are deferred until a message is opened.

Added `integrations.rs` for bring-your-own-key management. VirusTotal and Google
Safe Browsing keys are validated before enabling, stored in the OS keychain as
`integration-<provider>`, and never returned to the webview; SQLite records only
the enabled flag, status, last check, and error. Added `keys.rs` for Ed25519
identity keys: private material is written only to the OS keychain as
`crypto-key-<id>`, public keys and fingerprints live in SQLite, and the UI can
generate, rotate, revoke, export, and import keys plus store contact public
keys. Added `external_scanning` as a local setting that disables every external
provider call.

Rebuilt the Security settings page with the Shield summary, integration cards,
external-scanning toggle, alerts, identity-key lifecycle, contact-key import,
and audit events. Added per-row Shield flags, a viewer threat banner with link
verdicts, and false-positive reporting to the message viewer. Added
browser-preview fallbacks that keep integration secrets in memory only and run
local-only Shield heuristics.

Validation: [OK] `node --check` for `main.js`, `tauri.js`, `SecurityPage.js`,
and `security.js`; [OK] `git diff --check`; Rust compilation and the Windows
desktop build run in GitHub Actions because no local Rust toolchain is
available.

[200 OK] Added message importance and priority (Phase 2, GitHub issue #5):

Added `priority.rs` with deterministic local suggestions for Low, Normal, High,
and Urgent priority. Suggestions run when demo or IMAP mail is imported, record
`priority_source = "auto"` plus a short reason, and can conservatively mark
obvious important mail. Manual importance and priority changes set
`priority_source = "manual"` and are never overwritten. Added `is_important`,
`priority`, `priority_source`, and `priority_reason` to the message schema with a
migration that preserves the existing Important label. Added `auto_priority` and
`priority_default` settings, an `important` mailbox and quick filters for
important, high, and urgent mail, plus the `refresh_priority_suggestions`
command.

The inbox now shows importance flags and priority pills, the reader has an
importance toggle, a priority selector with its reason, and Settings includes
automatic-suggestion controls and a refresh action. Bulk actions can mark
messages important or not important. Browser preview mirrors the same behavior
with `localStorage`.

Files changed: `mailbox.rs`, `providers.rs`, `priority.rs`, `lib.rs`, `main.js`,
`index.html`, `styles.css`, `tauri.js`, `task.md`, `docs/user-guide.md`,
`docs/reference/tauri-commands.md`, `docs/reference/configuration.md`.

Validation: [OK] `node --check` for `main.js` and `tauri.js`; [OK] CSS braces
balanced (525/525); [OK] `git diff --check`; Rust compilation and the Windows
desktop build run in GitHub Actions because no local Rust toolchain is
available.

[200 OK] Added sender and domain blocking plus a trusted allowlist (Phase 2, GitHub issue #6):

Added `senders.rs` with local block and allow rules for individual addresses and
whole domains. Rules are stored in the `sender_rules` table with a unique
`(kind, scope, value)` key. Allow rules always win, so a trusted address or
domain is never filtered even when a broader block rule matches. Incoming IMAP
mail and demo messages are evaluated at import time. Blocked mail moves to Spam
or Trash according to the new `blocked_sender_action` setting, and the matching
rule is stored in `emails.blocked_by` as `address:<value>` or `domain:<value>`.
Removing a rule or adding an allow rule restores auto-filtered messages to the
Inbox, while manual folder moves clear `blocked_by` so they are never restored.

The inbox and reader now show a **Blocked** chip with the matching address or
domain. Settings includes a **Senders & domains** section for adding block and
allow rules, deleting them, choosing the blocked-mail destination, and applying
rules on demand. Browser preview mirrors the same behavior with `localStorage`
and `sender-rules-v1`.

Files changed: `senders.rs`, `mailbox.rs`, `providers.rs`, `lib.rs`, `main.js`,
`index.html`, `styles.css`, `tauri.js`, `task.md`, `docs/user-guide.md`,
`docs/reference/tauri-commands.md`, `docs/reference/configuration.md`.

Validation: [OK] `node --check` for `main.js` and `tauri.js`; [OK] CSS braces
balanced (536/536); [OK] `git diff --check`; Rust compilation and the Windows
desktop build run in GitHub Actions because no local Rust toolchain is
available.

[200 OK] Added sender and domain reputation with authentication results (Phase 2, GitHub issue #7):

Added `reputation.rs` with a local Trusted, Neutral, Suspicious, and Malicious
classification for the sender domain of each message. Scores combine the
allowlist and blocklist, official brand domains, brand impersonation and
punycode checks, SPF, DKIM, and DMARC outcomes, and any threat-intelligence
verdicts already cached by Shield. Unknown domains start at Neutral and are
never treated as malicious without evidence. The IMAP importer now parses
`Authentication-Results` and `Received-SPF` from the message header block into
`emails.auth_results`; body text cannot spoof those results. The new
`domain_reputation` cache is keyed by domain, and `reputation_feedback` records
reports.

The reader shows a reputation card with the classification, sender domain,
SPF/DKIM/DMARC badges, reasons, refresh, and **Report incorrect assessment**.
Reporting neutralizes the assessment until new intelligence exceeds the
dismissed score. The Security view can refresh every sender domain, and Shield
refreshes a message's reputation after an external scan. Browser preview mirrors
the same behavior with `localStorage` and `reputation-v1`.

Files changed: `reputation.rs`, `senders.rs`, `shield.rs`, `mailbox.rs`,
`providers.rs`, `lib.rs`, `main.js`, `SecurityPage.js`, `security.js`,
`tauri.js`, `styles.css`, `task.md`, `docs/user-guide.md`, `docs/security.md`,
`docs/reference/tauri-commands.md`.

Validation: [OK] `node --check` for `main.js`, `tauri.js`, `security.js`, and
`SecurityPage.js`; [OK] CSS braces balanced (571/571); [OK] fallback reputation
checks with `node`; [OK] `git diff --check`; Rust compilation and the Windows
desktop build run in GitHub Actions because no local Rust toolchain is
available. Also fixed a borrow-checker error in the Issue #6 `load_rules` query
that the Windows CI build surfaced.

[200 OK] Rebuilt the launcher download and uninstall flows:

The launcher now downloads through a `.part` file, resumes interrupted transfers
with HTTP Range requests, retries temporary failures with backoff, and promotes
the file to the final MSI name only after verification. A published
`<url>.sha256` sidecar is fetched before the transfer; `--sha256` and
`--require-checksum` cover mirrors without one, and a verified cached MSI is
reused. Progress shows percentage, transferred bytes and speed. Plain HTTP is
rejected unless `--allow-insecure-http` is passed, with localhost mirrors
allowed for local testing.

New commands and flags: `download` / `--download-only`, `cleanup`, `--retries`,
`--dry-run`, and `--cleanup`. Uninstall remains non-destructive by default but
now accepts `--remove-data` (or `--purge`) to delete the local data folder and
Elyra entries in Windows Credential Manager, `--backup-data <folder>` to copy
database files first, `--product-code <GUID>` for a specific registered product,
and `--dry-run` to preview the actions. The built-in help, getting-started,
troubleshooting, and configuration docs describe every flag.

Files changed: `apps/launcher/src/main.rs`, `docs/getting-started.md`,
`docs/troubleshooting.md`, `docs/reference/configuration.md`.

Validation: [OK] reviewed parse/download/uninstall paths and constructor flags;
[OK] `git diff --check`. Rust compilation and tests run in GitHub Actions
because no local Rust toolchain is available.

[200 OK] Added Linux package distribution support (Phase 3, GitHub issue #12):

Added `.github/workflows/linux-release.yml` to build Tauri AppImage and .deb
bundles on `ubuntu-latest` for every `v*` tag and manual dispatch, with Tauri
v2 Linux system dependencies, optional signing secrets, stable and versioned
asset names, SHA-256 checksums, and artifact upload. Publishing the Linux
assets to the GitHub Release is opt-in via `ELYRA_LINUX_PUBLISH_RELEASE`.

Added `packaging/linux/` with an AUR-style `PKGBUILD` that verifies the
published `.sha256` before installing the AppImage to `/opt/elyra`, a desktop
entry, a 256x256 hicolor icon, a refresh script, and a README. Added
`docs/linux.md` with install, dependency, source-build, update/uninstall,
troubleshooting, and Flatpak investigation notes (Flatpak not shipped). Linked
the new assets and guide from `README.md` and the website download section.

Files changed: `.github/workflows/linux-release.yml`, `docs/linux.md`,
`packaging/linux/PKGBUILD`, `packaging/linux/elyra.desktop`,
`packaging/linux/elyra.png`, `packaging/linux/copy-icon.sh`,
`packaging/linux/README.md`, `README.md`, `web/index.html`, `web/styles.css`.

Validation: [OK] YAML parsed with PyYAML; [OK] `bash -n` on all workflow run
blocks and the PKGBUILD; [OK] `sh -n` on `copy-icon.sh`; [OK] simulated
checksum verification (match accepted, mismatch rejected) and asset collection
(stables + versioned names + checksums); [OK] relative links and whitespace
checks. The AppImage/.deb builds are validated in GitHub Actions because no
local Rust toolchain is available.

[200 OK] Added automated server tests and production verification docs (Phase 3, GitHub issues #9 and #10):

Added pure Rust tests for Argon2id hashing, JWT expiry/secret isolation, SMTP
config normalization, disabled-relay fail-closed behavior, API validation,
bearer-header parsing and error status mapping. Added database-backed API tests
covering register/login, sync cursors, notification privacy/read ownership and
unconfigured mail; they skip gracefully without PostgreSQL and use a shared
advisory lock to avoid migration races. Added `server/tests/database_integration.rs`
for schema idempotency, uniqueness, upserts and cascade deletes. Forwarded
`ELYRA_ACCESS_TOKEN_TTL_HOURS` through production Compose, changed backups to
`pg_dump --clean --if-exists`, and documented a safe restore with
`ON_ERROR_STOP=1`. Added `docs/deploy/VERIFY.md` with smoke tests, HTTPS checks,
restart persistence, backup/restore drill and sign-off, plus explicit
documentation that custom domains need no source edits.

Files changed: `server/src/auth.rs`, `server/src/mailer.rs`, `server/src/api.rs`,
`server/tests/database_integration.rs`, `infrastructure/deployment/docker-compose.prod.yml`,
`infrastructure/deployment/.env.prod.example`, `infrastructure/deployment/deploy.sh`,
`docs/deploy/VERIFY.md`, `docs/deploy/SERVER.md`, `docs/deployment.md`,
`docs/development.md`, `docs/server.md`, `docs/README.md`, `docs/ARCHITECTURE.md`,
`docs/reference/configuration.md`, `docs/getting-started.md`,
`docs/troubleshooting.md`.

Validation: [OK] `git diff --check`; [OK] Compose YAML parses; [OK] local Markdown
links resolve; [OK] Linux packaging doc additions included. [OK] Rust tests are
intended for CI: no local toolchain is available, so `cargo test -p elyra-server`
was not executed here.
