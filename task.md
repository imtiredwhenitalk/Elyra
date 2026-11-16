### Features & Roadmap

* **UI/UX design** - create a UI/UX design and create some test emails to check [200 OK]
* **OAuth/Auth** - desktop registration/login plus Rust cloud session bridge [200 OK: desktop]; provider OAuth with PKCE [NEXT]
* **Add More UI/UX features** - spam, trash, settings, profile, command palette, bulk actions [200 OK]
* **Message Viewer Core** - full message rendering, attachments, reply/forward, starring [200 OK]
* **Email Management Actions** - move, delete, snooze, mark read/unread, bulk actions [200 OK]
* **Advanced Theming System** - Light, Dark, Midnight, OLED, four accents, density, reduce motion [200 OK]
* **Polished Responsive UI/UX** - responsive layouts, focus states, reduced motion, no broken selectors [200 OK]
* **Custom Elyra Domain Support** - display and manage `username@elyra.com` handles [200 OK: UI prototype]
* **Local Database Storage (SQLite)** - Rust schema migration, WAL, foreign keys, secure deletion, settings [200 OK]
* **Rust Mailbox Engine** - search, filters, stats, drafts, bulk actions, snooze, attachments, sent mail, export [200 OK]
* **Email Provider Engine (IMAP/SMTP & API)** - SMTP outbound send [200 OK]; IMAP inbox sync for Gmail, Outlook, iCloud, Yahoo, Fastmail, Proton Bridge and custom servers [200 OK]; provider OAuth APIs [NEXT]
* **Crypto Engine & Key Management** - AES-256-GCM, fingerprints, tamper detection, Rust security lab [200 OK]
* **OS Keychain Token Storage** - cloud sessions and IMAP passwords stored with `keyring-rs` in the Windows Credential Manager [200 OK]
* **Elyra Backend Service (Rust)** - account routing, metadata sync, notifications, REST API [200 OK]; WebSocket live updates [NEXT]
* **Server Mail Relay** - registered users can send to any external address through the server when no local SMTP account is configured [200 OK]
* **First-Run Registration** - registration onboarding modal, configurable server URL, and Continue offline [200 OK]
* **PostgreSQL & Server Infrastructure** - schema, migrations, Docker deployment [200 OK]; one-command `deploy.sh`, optional Caddy HTTPS, and GitHub Actions SSH deploy [200 OK]
* **Rust Diagnostics** - engine/Tauri/SQLite versions, database size, changelog [200 OK]
* **Consistent Icon System** - inline SVG sprite using `currentColor`; removed image-based icons [200 OK]
* **Windows Delivery** - MSI/NSIS release workflow, stable asset names, SHA-256 checksums [200 OK]
* **Portable Launcher** - download, verify, install, update, launch, uninstall [200 OK]
* **Public Website** - responsive marketing/download page deployed to GitHub Pages [200 OK]
* **Project Documentation** - structured `docs/` set covering install, user guide, architecture, server/API, security, development, deployment, configuration, Tauri commands, and troubleshooting [200 OK]

### Changes (Tracking Log)

* Added the Rust mailbox engine and moved local mailbox behavior out of browser JavaScript.
* Added the Rust cloud bridge with token-hiding session views.
* Replaced image-based icons with one inline SVG sprite and removed unused assets.
* Rebuilt the desktop UI with themes, accents, density, bulk actions, command palette, and diagnostics.
* Added Rust diagnostics and changelog commands.
* Added the Windows release workflow, stable download names, and checksum publishing.
* Rewrote the portable launcher as a verifier/installer CLI.
* Rewrote the public website and added `web/app.js` plus a matching favicon.
* Corrected installer and data-path documentation.
* Moved the database to the app-local identifier folder and moved blocking network commands off the main thread.
* Moved Elyra Cloud sessions and IMAP passwords into the OS keychain (`keyring-rs`); SQLite now only holds legacy migration data.
* Added IMAP account management, provider presets, connection tests, and INBOX sync with local read/starred state preserved.
* Extended outbound SMTP send to use a selected account and its keychain password, with environment variables as fallback.
* Added first-run account registration onboarding and a configurable Elyra server URL.
* Added the server SMTP relay so registered users can send mail to external addresses without local SMTP setup.
* Added the production deploy script, optional Caddy HTTPS profile, GitHub Actions SSH deploy workflow, and `docs/deploy/SERVER.md`.
* Published the full documentation set under `docs/` and reconciled `README.md`, `ai.md`, and `docs/ARCHITECTURE.md` with the shipped static frontend and repository layout.
