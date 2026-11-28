**Elyra** is a privacy-focused email client designed to bring your inbox, security, and communication into one place.

Elyra connects multiple email providers such as Gmail, Proton Mail, Outlook, and others, while intelligently filtering and prioritizing important messages. The goal is to make email communication simpler, more private, and more secure.

### Features

* **Unified Inbox** — manage emails from different providers in one application.
* **Smart Filtering** — separate important emails, services, notifications, and other messages.
* **Desktop-first experience** — important emails and notifications are always within reach.
* **Privacy-focused architecture** — minimize unnecessary access to your personal data.
* **Strong security** — protection against unauthorized access and common attack vectors.
* **Encrypted communication** — secure email communication where supported.
* **Cross-provider messaging** — communicate with users across different email services.
* **Fast & lightweight** — designed to stay responsive even with large inboxes. 

### Rules 

* **architecture permission** - don't touch architecture without my permission, u can change code u can change documentation but don't architecture 
* **After you done something** - if you done something u just mark it like that [200 OK] , [OK] 

* **architecture permission v2.0** - If anything is missing, you can add or create it, but please include a "changes" tab for future reference (to track what was modified, added, etc.).

### Language u need to use in project 

## Desktop / app Rust + Tauri / fast, low RAM usage, good security baseline
## UI / static HTML + CSS + JavaScript / the shipped 0.4.0 interface (React + TypeScript is the intended future direction, not implemented)
## Backend / Rust / security , concurrency, productivity
## Crypto / Rust / memory safety + strange crypto-ecosystem
## Local DB / SQLite / local cache , index , settings 
## API / REST (WebSocket is planned, not implemented) / syncronizating and real-time event's
## Server DB / PostgreSQL / account , metadata , settings 
## Deployment / Docker / simple backend deployment
## Secret / OS Keychain , security storage / Tokens don't just sit in file

### Project Layout & Directory Structure

This is the actual 0.4.0 layout. See [`docs/en/architecture.md`](docs/en/architecture.md)
for the full explanation. Documentation is available in English (`docs/en/`) and
Ukrainian (`docs/ua/`); choose a language at [`docs/README.md`](docs/README.md).

Elyra (Repository Root)
├── ai.md                         # Project rules, intended stack, and context
├── README.md                     # Project overview and downloads
├── task.md                       # Feature checklist and roadmap status
├── changes.md                    # Chronological implementation log
├── Cargo.toml                    # Rust workspace (crypto, server, launcher, desktop)
├── apps/                         # Client applications
│   ├── desktop/                  # Tauri 2 desktop app (Rust core + static UI)
│   │   ├── src/                  # static HTML/CSS/JS UI
│   │   │   ├── index.html        # shell and inline SVG sprite
│   │   │   ├── styles.css        # themes, accents, density, responsive rules
│   │   │   ├── main.js           # state, rendering, modals, command palette
│   │   │   ├── components/       # CryptoCard.js
│   │   │   ├── pages/            # SecurityPage.js
│   │   │   └── services/         # tauri.js invoke wrapper + browser fallback
│   │   ├── src-tauri/            # Rust core and Tauri config
│   │   ├── services/             # sendemail.rs compiled into the desktop crate
│   │   └── package.json          # Tauri CLI only; no bundler
│   └── launcher/                 # Portable Windows installer/updater CLI (Rust)
├── packages/                     # Shared Workspace Packages
│   ├── crypto/                   # AES-256-GCM primitives (Rust)
│   ├── types/                    # Shared TypeScript interfaces & types
│   └── ui/                       # Framework-neutral UI primitives (not yet wired into desktop)
├── server/                       # Backend API server (Rust, Axum)
│   ├── src/api.rs                # REST routes
│   ├── src/auth.rs               # Argon2id + JWT authentication
│   ├── src/database.rs           # PostgreSQL schema
│   ├── src/sync.rs               # Sync cursors
│   ├── src/users.rs              # User accounts
│   └── src/mailer.rs             # Optional SMTP relay
├── web/                          # Static marketing/download website
├── docs/                         # Documentation (two languages)
│   ├── README.md                 # Language selector
│   ├── en/                       # English documentation (canonical)
│   │   ├── README.md             # Documentation index
│   │   ├── getting-started.md    # Install and first run
│   │   ├── user-guide.md         # Interface and features
│   │   ├── architecture.md       # System architecture
│   │   ├── server.md             # Server and API reference
│   │   ├── security.md           # Security model and threat model
│   │   ├── development.md        # Build, test, release
│   │   ├── deployment.md         # Production deployment overview
│   │   ├── troubleshooting.md    # Symptom → fix guide
│   │   ├── reference/            # configuration.md, tauri-commands.md
│   │   ├── deploy/               # SERVER.md, VERIFY.md
│   │   ├── architecture/         # short directory scope note
│   │   ├── crypto/               # crypto layer notes
│   │   └── security/             # security architecture note
│   └── ua/                       # Ukrainian documentation (same structure)
├── infrastructure/               # DevOps & Deployment
│   ├── build/                    # development helper scripts
│   ├── docker/                   # development Compose stack
│   ├── deployment/               # production Compose, deploy.sh, Caddyfile
│   └── web/                      # Nginx config for the production site
└── .github/workflows/            # CI, release, Pages, and server deploy

### Architecture Elyra

                         ┌─────────────────────┐
                         │       Elyra         │
                         │    Desktop App      │
                         └──────────┬──────────┘
                                    │
                       ┌────────────▼────────────┐
                       │      Tauri / Rust       │
                       │   Application Core      │
                       └───────┬─────────┬───────┘
                               │         │
                 ┌─────────────┘         └─────────────┐
                 ▼                                       ▼
        ┌─────────────────┐                   ┌─────────────────┐
        │  Email Engine   │                   │ Crypto Engine   │
        │                 │                   │                 │
        │ Gmail (IMAP)    │                   │ AES-256-GCM     │
        │ Proton Bridge*  │                   │ SHA-256         │
        │ Outlook (IMAP)  │                   │ Key in memory   │
        │ IMAP/SMTP       │                   │ (keychain)      │
        └────────┬────────┘                   └─────────────────┘
                 │
                 ▼
        ┌─────────────────┐
        │  Local Storage  │
        │                 │
        │ SQLite          │
        │ Email cache     │
        │ Search index    │
        │ Settings        │
        └─────────────────┘

                         │ HTTPS
                         ▼

              ┌─────────────────────┐
              │    Elyra Backend    │
              │        Rust         │
              └──────────┬──────────┘
                         │
             ┌───────────┼───────────┐
             ▼           ▼           ▼
        PostgreSQL    REST API    Argon2id/JWT