# Elyra Security Architecture

## Security baseline

The desktop client uses defense in depth around a local-first trust boundary:

- Rust/Tauri is the privileged boundary. Browser JavaScript can request allowlisted commands but does not receive filesystem access.
- SQLite is created under the platform app-local data directory, with foreign keys, WAL journaling, and `secure_delete` enabled for every connection.
- Sensitive cryptographic operations run in Rust with AES-256-GCM, OS randomness, fresh nonces, strict key sizes, and authentication-tag validation.
- Security posture checks are exposed through `get_security_status`; the Security Lab renders pass/fail state and does not fabricate a healthy result when the backend is unavailable.
- Audit records are stored locally in `audit_events`, contain event type/outcome/details only, and are capped at 500 records. Keys, tokens, message bodies, and passwords must never be written to the audit log.

## Threat model

Protected against:

- Accidental plaintext exposure through the crypto boundary and malformed ciphertext.
- Corruption caused by invalid SQLite relationships or unsafe local database settings.
- Unbounded local audit growth.
- UI-only security claims when the Rust command is unavailable.

Not yet protected against:

- A compromised operating-system user or malware with access to the app data directory.
- Account takeover, OAuth token theft, or remote mailbox abuse. OAuth must use Authorization Code + PKCE and OS keychain storage before production use.
- Full forensic tamper resistance. Local SQLite audit records are evidence for troubleshooting, not a trusted remote security log.

## Required controls before production

1. Store account refresh tokens and encryption keys in the platform credential store (Windows Credential Manager, macOS Keychain, Linux Secret Service), never in SQLite, localStorage, URLs, or logs.
2. Replace placeholder SMTP/OAuth flows with TLS certificate validation, PKCE, state/nonce validation, token rotation, and least-privilege scopes.
3. Add authenticated database export/backup, secure deletion UX, lock-on-idle, and a recovery procedure.
4. Add structured redaction tests and negative tests for every Tauri command boundary.
5. Ship remote, append-only audit forwarding only after user consent and with PII minimization.

## Operational checks

The Security Lab runs the local SQLite posture check and displays the bounded audit trail. A `protected` result means the configured local controls passed; it does not mean the device or external mail provider is secure. Release validation must include `cargo test`, `cargo check`, JavaScript syntax checks, dependency auditing, and a manual review of Tauri capabilities/CSP.
