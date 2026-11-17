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
