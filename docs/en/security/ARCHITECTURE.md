> 🌐 **English** · [Українська](../../ua/security/ARCHITECTURE.md)

# Elyra Security Architecture

## Security baseline

The desktop client uses defense in depth around a local-first trust boundary:

- Rust/Tauri is the privileged boundary. Browser JavaScript can request allowlisted commands but does not receive filesystem access.
- SQLite is created at `%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3` on Windows, with foreign keys, WAL journaling, and `secure_delete` enabled for every connection.
- Sensitive cryptographic operations run in Rust with AES-256-GCM, OS randomness, fresh nonces, strict key sizes, and authentication-tag validation.
- Security posture checks are exposed through `get_security_status`; the Security Lab renders pass/fail state and does not fabricate a healthy result when the backend is unavailable.
- Audit records are stored locally in `audit_events`, contain event type/outcome/details only, and are capped at 500 records. Keys, tokens, message bodies, and passwords must never be written to the audit log.

### Secret storage

Cloud sessions and IMAP/SMTP passwords are stored in the OS keychain through
`keyring-rs` (Windows Credential Manager on Windows). The keychain service is
`com.elyra.desktop`; cloud sessions use the `cloud_session` entry, each mail
account uses `imap-account-<id>`, and provider OAuth tokens use
`oauth-account-<id>`. SQLite only holds the legacy
`settings.cloud_session` row, which is migrated to the keychain on first read and
then deleted. Secrets are never written to the webview, logs, or audit trail.

### Server trust boundary

The Elyra server stores only account metadata, sync cursors, notifications, and
the optional SMTP relay configuration. It never receives desktop encryption keys
or IMAP passwords. When a signed-in user has no local SMTP account, outbound
mail is sent through the server relay and therefore transits the server's SMTP
provider; the message body is not persisted by the Elyra server. Users who do
not want that path can configure their own IMAP/SMTP account or continue
local-only. The relay is disabled unless the operator sets `ELYRA_SMTP_HOST`.

## Threat model

Protected against:

- Accidental plaintext exposure through the crypto boundary and malformed ciphertext.
- Plaintext credential storage: cloud tokens and mailbox passwords live in the OS credential store, not SQLite.
- Corruption caused by invalid SQLite relationships or unsafe local database settings.
- Unbounded local audit growth.
- UI-only security claims when the Rust command is unavailable.

Not yet protected against:

- A compromised operating-system user or malware with access to the app data directory or the unlocked credential store.
- Account takeover, OAuth token theft, or remote mailbox abuse. Provider OAuth uses Authorization Code + PKCE and keychain token storage. IMAP supports both app passwords and `AUTHENTICATE XOAUTH2` over TLS/STARTTLS; SMTP XOAUTH2 is still out of scope.
- Full forensic tamper resistance. Local SQLite audit records are evidence for troubleshooting, not a trusted remote security log.

## Required controls before production

1. Store account refresh tokens and encryption keys in the platform credential store (Windows Credential Manager, macOS Keychain, Linux Secret Service), never in SQLite, localStorage, URLs, or logs. Cloud sessions, IMAP passwords, and provider OAuth token sets are already keychain-backed.
2. Add SMTP XOAUTH2 with token refresh and least-privilege SMTP scopes. The IMAP OAuth flow already uses PKCE, state validation, TLS certificate validation, and token refresh; IMAP/SMTP already require TLS or STARTTLS with certificate validation.
3. Add authenticated database export/backup, secure deletion UX, lock-on-idle, and a recovery procedure.
4. Add rate limiting and abuse controls to registration and the server mail relay before opening the service to the public.
5. Add structured redaction tests and negative tests for every Tauri command boundary.
6. Ship remote, append-only audit forwarding only after user consent and with PII minimization.

## Operational checks

The Security Lab runs the local SQLite posture check and displays the bounded audit trail. A `protected` result means the configured local controls passed; it does not mean the device or external mail provider is secure. Release validation must include `cargo test`, `cargo check`, JavaScript syntax checks, dependency auditing, and a manual review of Tauri capabilities/CSP.