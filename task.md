### Features & Roadmap

* **UI/UX design** - create a UI/UX design and create some test emails to check [200 OK]
* **OAuth/Auth** - need create Auth and Authentification [NEXT: backend integration]
* **Add More UI/UX features** - add more features and components like (spam , trash) add settings and profile [200 OK]
* **Message Viewer Core** - implement full email message rendering (HTML/Text body, inline images, attachments viewer) [200 OK]
* **Email Management Actions** - add ability to move messages (Spam, Trash, Custom Folders), hard delete, and set priority / flag emails [200 OK: local prototype]
* **Advanced Theming System** - expand settings with multiple UI themes (Dark, Light, Midnight, OLED, Custom Accents) and seamless switching [200 OK]
* **Polished Responsive UI/UX** - refine layouts, fix alignment/scrolling issues, smooth animations, and zero broken elements [200 OK]
* **Custom Elyra Domain Support** - display and manage custom email domain handles for registered users (e.g. `username@elyra.com`) [200 OK: UI prototype]
* **Local Database Storage (SQLite)** - set up SQLite schema via Rust/Tauri to cache emails, folders, contacts, and user settings locally [200 OK]
* **Email Provider Engine (IMAP/SMTP & API)** - implement Rust email fetching engine for standard IMAP/SMTP and provider APIs (Gmail, Outlook)
* **Crypto Engine & Key Management** - build end-to-end encryption/decryption module in Rust, PGP/custom key generation, and signature verification
* **OS Keychain Token Storage** - securely store OAuth access/refresh tokens and encryption keys using OS Keychain (`keyring-rs`)
* **Elyra Backend Service (Rust)** - develop backend API endpoints for account routing, metadata sync, and real-time updates via WebSockets
* **PostgreSQL & Server Infrastructure** - database schema for user accounts, metadata, and Docker deployment setup

### Changes (Tracking Log)

* Added **Message Viewer Core** for full body and attachment previewing.
* Added **Email Management Actions** (Move, Delete, Flag/Priority).
* Expanded **Theming System** in Settings (Multiple visual styles).
* Added **UI/UX Refinement** task for pixel-perfect layout and animations.
* Integrated **Custom Domain Handling** (`@elyra.com`) into account and email views.
* Added **SQLite Storage** task for local caching and offline capabilities.
* Added **Email Engine (IMAP/SMTP)** task for real email service connections.
* Added **Crypto Engine** task for end-to-end encryption and key handling.
* Added **Keychain Integration** task for secure local secrets management.
* Added **Rust Backend & PostgreSQL** tasks for server-side architecture.