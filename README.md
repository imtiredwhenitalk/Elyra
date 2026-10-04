# Elyra

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

### Vision

Elyra aims to rethink the traditional email experience by combining the convenience of a modern inbox with privacy and security as first-class principles.

> **Your email. Your privacy. Your control.**

### Status

Elyra is currently under active development. Features, architecture, and APIs may change as the project evolves.

### Desktop build

Use `npm.cmd run check` for the frontend syntax check and `npm.cmd run build` to create the Windows Tauri installer. Sending mail requires `ELYRA_SMTP_FROM`, `ELYRA_SMTP_HOST`, `ELYRA_SMTP_USERNAME`, and `ELYRA_SMTP_PASSWORD` in the process environment.

### Server and shared packages

The Rust workspace now includes the reusable [`elyra-crypto`](packages/crypto)
package and the authenticated [`server`](server) service. The server uses
PostgreSQL for account metadata and sync cursors, Argon2id for password
hashing, and signed access tokens. It never receives local encryption keys.

Run `cargo run -p elyra-server` after starting the PostgreSQL service and
configuring the variables from [`server/.env.example`](server/.env.example).
Shared TypeScript contracts live in [`packages/types`](packages/types), and
framework-neutral UI primitives live in [`packages/ui`](packages/ui).
