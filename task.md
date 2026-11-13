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
* **Email Provider Engine (IMAP/SMTP & API)** - SMTP outbound send [200 OK]; IMAP fetch and Gmail/Outlook APIs [NEXT]
* **Crypto Engine & Key Management** - AES-256-GCM, fingerprints, tamper detection, Rust security lab [200 OK]
* **OS Keychain Token Storage** - store OAuth tokens and keys with `keyring-rs` [NEXT]
* **Elyra Backend Service (Rust)** - account routing, metadata sync, notifications, REST API [200 OK]; WebSocket live updates [NEXT]
* **PostgreSQL & Server Infrastructure** - schema, migrations, Docker deployment [200 OK]
* **Rust Diagnostics** - engine/Tauri/SQLite versions, database size, changelog [200 OK]
* **Consistent Icon System** - inline SVG sprite using `currentColor`; removed image-based icons [200 OK]
* **Windows Delivery** - MSI/NSIS release workflow, stable asset names, SHA-256 checksums [200 OK]
* **Portable Launcher** - download, verify, install, update, launch, uninstall [200 OK]
* **Public Website** - responsive marketing/download page deployed to GitHub Pages [200 OK]

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
