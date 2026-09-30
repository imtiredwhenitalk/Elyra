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

## Desktop / app Rust + Tauri + TypeScript / fast, low RAM usage, good security baseline
## UI / React + TypeScript / Good loking interface
## Backend / Rust / security , concurrency, productivity
## Crypto / Rust / memory safety + strange crypto-ecosystem
## Local DB / SQLite / local cache , index , settings 
## API / REST + WebSocket / syncronizating and real-time event's 
## Server DB / PostgreSQL / account , metadata , settings 
## Deployment / Docker / simple backend deployment
## Secret / OS Keychain , security storage / Tokens don't just sit in file

### Project Layout & Directory Structure

Elyra (Repository Root)
├── Claude.md                     # System instructions & context for AI
├── README.md                     # Project overview
├── apps/                         # Client applications
│   ├── desktop/                  # Primary Desktop App (Tauri + React + TS)
│   │   ├── src/                  # React UI components & views
│   │   ├── src-tauri/            # Rust Backend for Desktop (Tauri Core)
│   │   ├── components/           # Reusable UI components
│   │   ├── hooks/                # Custom React hooks
│   │   ├── pages/                # App views/routes
│   │   ├── services/             # Client API & WebSocket services
│   │   └── package.json
│   └── src-tauri/                # Shared/standalone Tauri templates
├── packages/                     # Shared Workspace Packages
│   ├── crypto/                   # Encryption & key management core (Rust)
│   ├── types/                    # Shared TypeScript interfaces & types
│   └── ui/                       # Shared UI Design System / React components
├── server/                       # Backend API & Sync Server (Rust)
│   ├── api/                      # REST & WebSocket endpoints
│   ├── auth/                     # OAuth2 & Session authentication
│   ├── database/                 # PostgreSQL connections & queries
│   ├── sync/                     # Multi-provider email synchronization
│   └── users/                    # User account management
├── docs/                         # System Documentation
│   ├── architecture/             # Data flow & component diagrams
│   ├── crypto/                   # Encryption specs & key schemes
│   └── security/                 # Threat models & security rules
└── infrastructure/               # DevOps & Deployment
    ├── deployment/               # CI/CD pipelines & release scripts
    └── docker/                   # Dockerfiles & docker-compose setups

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
        │ Gmail           │                   │ Encryption      │
        │ Proton*         │                   │ Signatures      │
        │ Outlook         │                   │ Key management  │
        │ IMAP/SMTP       │                   │                 │
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
        PostgreSQL    WebSocket    Auth