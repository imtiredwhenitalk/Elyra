> 🌐 **English** · [Українська](../ua/security.md)

# Security and privacy

Elyra is designed around a local-first trust boundary: mail stays on the user's
device, privileged operations run in Rust, and the optional cloud service holds
only account metadata, sync cursors, notifications, and relay configuration.

This page describes the security model as implemented in 0.5.0, including what
is **not** protected yet. It is a technical document, not a certification.

## Security goals

1. Keep mailbox content and IMAP/SMTP passwords on the user's machine.
2. Keep secrets out of SQLite, `localStorage`, URLs, logs, and the webview.
3. Require authenticated encryption for local cryptographic operations.
4. Require TLS or STARTTLS for mail protocols.
5. Keep the server's data set minimal and scoped per user.
6. Report security state honestly instead of showing a fake pass.
7. Fail closed when configuration is invalid.

## Assets

| Asset | Where it lives | Sensitivity |
| --- | --- | --- |
| Mailbox messages and attachments | Local SQLite + filesystem | High |
| IMAP/SMTP passwords | OS keychain | Critical |
| Elyra Cloud access token | OS keychain | Critical |
| Elyra Cloud password | Server Argon2id hash | Critical |
| Desktop encryption keys (Security Lab) | Memory only | High |
| Ed25519 identity private keys | OS keychain | Critical |
| Integration API keys (VirusTotal, Google Safe Browsing) | OS keychain | High |
| Account metadata (hosts, addresses) | Local SQLite and server `users` | Medium |
| Notifications | Server PostgreSQL | Medium |
| Audit events | Local SQLite | Low/Medium |
| Relay SMTP credentials | Server environment | Critical |

## Trust boundaries

### Webview ↔ Rust

The webview is treated as untrusted input. It can call only the commands
registered in `apps/desktop/src-tauri/src/lib.rs`. It cannot open SQLite, read
files, access the keychain, or perform raw network requests. Rust validates
arguments and returns errors.

`tauri.conf.json` currently sets `csp: null` and `withGlobalTauri: true`. This is
a deliberate 0.5.0 trade-off for the static frontend and Tauri API access. A
strict Content Security Policy and reduced global surface are recommended before
treating the webview as a hardened boundary.

### Desktop ↔ OS keychain

Secrets are stored with `keyring` 3 using the `windows-native` backend, which
maps to Windows Credential Manager.

- Service: `com.elyra.desktop`
- Cloud session entry: `cloud_session`
- Mail account entry: `imap-account-<id>`
- Provider OAuth token entry: `oauth-account-<id>`
- Integration API key entry: `integration-<provider>` (`virustotal`,
  `google_safe_browsing`)
- Identity private key entry: `crypto-key-<id>`

The keychain is protected by the operating system user context. Elyra does not
add a second application-level encryption layer to keychain values.

### Desktop ↔ mail provider

- IMAP: `imap` 2.4 over `native-tls`, TLS or STARTTLS only.
- SMTP: `lettre` with rustls, TLS or STARTTLS only.
- Plaintext mail protocols are not supported.
- App passwords and OAuth 2.0 Authorization Code + PKCE are supported for IMAP.
  OAuth uses an ephemeral loopback redirect, validates `state`, and keeps tokens
  out of the webview; see [Provider OAuth](oauth.md).
- SMTP XOAUTH2 is not implemented; OAuth accounts still need an app password or
  the Elyra relay for sending.

### Desktop ↔ Elyra server

- HTTPS in production.
- Bearer JWT access tokens.
- The server never receives IMAP passwords, encryption keys, or mailbox
  contents.
- Relayed messages transit the server's SMTP provider; the Elyra server does not
  persist the body.

### Public internet ↔ server

- Caddy terminates TLS (optional profile) and forwards to Nginx.
- Nginx serves the static site and proxies `/api/` to the API container.
- PostgreSQL, Redis, and the API are on a private Docker network.
- Only the website port is published by default; with TLS, only 80/443.

## Local data protection

### SQLite

Database path: `%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3`.

Every connection enables:

```sql
PRAGMA foreign_keys = ON;   -- relationship integrity
PRAGMA journal_mode = WAL;  -- crash safety and concurrency
PRAGMA secure_delete = ON;  -- overwrite deleted content
```

`secure_delete` reduces recoverability of deleted rows but is not full-disk
encryption. Filesystem-level access by the same OS user, backups, volume
snapshots, or forensic tools can still expose data.

### Keychain

- Cloud session and mail passwords are keychain-backed.
- The legacy `settings.cloud_session` row is migrated to the keychain on first
  read and then deleted.
- SQLite stores account metadata (host, port, username) but not passwords.
- Removing an account deletes its keychain entry.
- Signing out deletes the cloud session entry.

### Audit log

`audit_events` is a local troubleshooting log:

- Records event type, outcome, and details.
- Capped at 500 rows.
- Must never contain keys, tokens, passwords, or message bodies.
- Is not a tamper-proof or remote security log.

### HTML and content rendering

The reader displays message text, not raw HTML. Text is escaped before being
inserted into the DOM. This prevents provider-supplied HTML and scripts from
executing in the webview. Attachments are shown as metadata and risk cards;
Elyra does not open or execute attachments in the webview.

## Cryptographic controls

The desktop crypto boundary is `apps/desktop/src-tauri/src/crypto.rs`, backed by
the reusable `elyra-crypto` package.

| Control | Implementation |
| --- | --- |
| Cipher | AES-256-GCM |
| Key size | 32 bytes, generated with `OsRng` |
| Nonce | 96-bit, fresh for every encryption |
| Payload | Base64 of nonce + ciphertext + authentication tag |
| Integrity | GCM authentication tag verified on decrypt |
| Fingerprint | SHA-256, display/comparison only |
| Key lifetime | Zeroized on drop in `elyra-crypto` |
| Failure behavior | Invalid key, malformed payload, wrong key, or tampered ciphertext returns an error |

The Security Lab uses a known test message and keeps generated keys in memory.
It demonstrates the primitive; it is not a production key-management service.

### Identity keys (Ed25519)

`apps/desktop/src-tauri/src/keys.rs` manages local identity keys.

- Generation uses `ed25519-dalek` with OS randomness.
- The 32-byte private key is written only to the OS keychain as
  `crypto-key-<id>`. SQLite stores the label, public key, SHA-256 fingerprint,
  status, and timestamps.
- The webview can export only the armored public key and can import contact
  public keys. No command returns private material.
- Rotation creates a new active key, marks the previous key `retired`, and
  deletes the previous keychain entry.
- Revocation marks the key `revoked` and deletes its keychain entry. Revocation
  is local and cannot be undone.
- Contact public keys are stored in SQLite with their fingerprints. Import
  validates the base64 payload and the Ed25519 point.
- Identity keys are the foundation for signing and future end-to-end
  encryption. Message encryption and key exchange are not implemented yet.

### Elyra Shield and threat intelligence

`apps/desktop/src-tauri/src/shield.rs` performs local analysis in Rust:

- Sender display-name and address checks against official brand domains,
  including punycode and lookalike normalization.
- Urgency and threatening language, credential or payment requests, dangerous
  attachment extensions, and visible-link versus destination mismatches.
- A 0–100 score maps to Safe, Suspicious, or Malicious and is stored per
  message with the reasons.
- Local analysis runs at IMAP import and when a message is first opened.
- False-positive reports are stored locally and shown in the UI.

`apps/desktop/src-tauri/src/threats.rs` performs optional external checks:

- Links are extracted from plain text and HTML anchors.
- VirusTotal uses `GET /api/v3/urls/{base64url}` with the user's `x-apikey`.
- Google Safe Browsing uses `POST /v4/threatMatches:find?key=...` and batches
  URLs.
- VirusTotal requests are throttled to one every 16 seconds on the free tier.
- Verdicts are cached in SQLite for 24 hours when known and 1 hour when
  unknown.
- Provider errors, rate limits, and missing keys are reported per link; they do
  not block message display.
- `settings.external_scanning = "false"` disables all external calls. Local
  Shield checks still run.
- External providers receive the message URLs, not the message body. The URLs
  themselves can still reveal sensitive information, so leave external scanning
  off if that is a concern.

`apps/desktop/src-tauri/src/reputation.rs` classifies sender domains:

- SPF, DKIM, and DMARC results are parsed from the message header block only;
  body text cannot spoof them.
- Scores combine allow and block rules, official brand domains, impersonation
  and punycode, authentication failures, and cached threat-intelligence verdicts.
- Unknown domains stay Neutral. The allowlist is always Trusted, and a report
  neutralizes a disputed assessment until new intelligence raises the score.
- Results are cached per domain in SQLite and can be refreshed on demand.

`apps/desktop/src-tauri/src/attachments.rs` analyzes attachments:

- Type detection reads magic bytes and structural markers, then compares the
  result with the declared MIME type and the file extension.
- Local scoring raises risk for executables, scripts, disk images, macro-enabled
  Office files, archives, HTML and SVG, double extensions, bidirectional-override
  characters in the name, and content/declared-type mismatches.
- Local analysis runs at import and on demand. It never executes, opens, or
  previews a file.
- ClamAV is opt-in. When enabled, `clamscan` runs on a temporary copy of stored
  content; the temporary file is removed afterward. Missing or failing ClamAV is
  reported as a reason and never blocks the mailbox.
- VirusTotal file analysis is opt-in and requires a stored API key plus explicit
  per-file consent. Uploads are size-limited and share the 16-second VirusTotal
  throttle. Clean VirusTotal results never downgrade a local warning.
- Results are stored per attachment in SQLite with a SHA-256 hash, detected type,
  kind, risk, score, reasons, source, and timestamp. Shield reuses the stored
  risk when scoring a message.

### What is not encrypted

- The SQLite mailbox database is not encrypted at rest by Elyra.
- Local message bodies, subjects, and attachments are stored as SQLite data.
- Backups and exports contain plaintext message content.
- The server's PostgreSQL data is not encrypted by the application. Use
  filesystem or volume encryption if required.

Full mailbox encryption at rest requires a key-management design and is a
future milestone.

## Server security

### Authentication

- Passwords hashed with Argon2id and random salts.
- JWT HS256 access tokens signed with `ELYRA_JWT_SECRET`.
- Secret must be at least 32 characters; the server refuses to start otherwise.
- Tokens expire after 24 hours by default.
- Server accounts still have no refresh tokens, revocation list, or password reset.
  OAuth provider sessions use provider refresh tokens and revocation.

### Authorization

- Every protected route verifies the bearer token and derives the user id from
  the token subject.
- Notifications are scoped to `recipient_id`; marking another user's
  notification returns `404`.
- `GET /v1/users/me` returns only the caller's record.
- Sync cursors are keyed by user id.

### Input validation

| Input | Limit |
| --- | --- |
| Email | Contains `@`, max 320 characters |
| Password | Min 12 characters |
| Display name | Non-empty, max 120 characters |
| Notification title | Non-empty, max 160 characters |
| Notification body | Non-empty, max 4000 characters |
| Mail subject | Non-empty, max 200 characters |
| Mail body | Non-empty, max 100 000 characters |

Database errors are logged server-side and returned as a generic
`internal_error`.

### Relay

- Opt-in through `ELYRA_SMTP_HOST`.
- Requires a valid `ELYRA_SMTP_FROM` at startup.
- Sends from the relay address with `Reply-To` set to the authenticated user.
- Does not persist the message body.
- Any registered user can send. There is no rate limit, quota, or abuse
  detection.

### Deployment hardening

The production stack follows these defaults:

- PostgreSQL and Redis are not published.
- The API is not published.
- The website binds to `127.0.0.1` when Caddy is enabled.
- Caddy adds HSTS, `X-Content-Type-Options: nosniff`, `Referrer-Policy`, and
  removes the `Server` header.
- No production secret has a repository default; Compose fails if a required
  value is missing.
- The API container runs as UID/GID `10001`.

## Threat model

### Protected against

| Threat | Control |
| --- | --- |
| Plaintext credential storage | Keychain for cloud sessions and mail passwords |
| Accidental token exposure to the webview | Session view excludes the access token |
| Malformed or tampered ciphertext | AES-GCM authentication tag validation |
| Plaintext mail protocol downgrade | TLS/STARTTLS only |
| SQL injection in mailbox queries | Parameterized SQL |
| Cross-user notification access | Recipient scoping on read and update |
| Password database leak | Argon2id hashes, not plaintext |
| Missing server secrets | Startup and Compose fail closed |
| UI faking a healthy security state | Security status comes from Rust; errors are shown |
| Unbounded local audit growth | 500-row cap |
| Installer tampering in transit | SHA-256 verification by the launcher |

### Not protected against

| Threat | Status |
| --- | --- |
| Compromised OS user or malware with access to the app data directory | Not protected. Use OS account hygiene, disk encryption, and endpoint protection |
| Malware that can read the unlocked Windows Credential Manager | Not protected by Elyra |
| Account takeover at the mail provider | Depends on provider controls; use strong unique app passwords and 2FA |
| OAuth token theft | Access and refresh tokens stay in the OS keychain and never reach the webview; malware that can read an unlocked keychain is still out of scope |
| Server database compromise | Server stores metadata and notifications; mail bodies are not stored, but account metadata is exposed |
| Relay abuse | No rate limiting or quotas yet |
| Phishing or malicious message content | Shield adds local heuristics and optional reputation checks, but no filter is complete; social engineering remains possible |
| Denial of service | No rate limits on registration or API routes |
| Supply-chain compromise | Reduced, not eliminated: the workspace `Cargo.lock` is current, Rust CI uses `--locked`, and weekly Dependabot PRs are enabled; release workflows may still resolve fresh versions |
| Full forensic tamper resistance | Local audit log is not tamper-evident |
| Metadata leakage to mail providers | IMAP/SMTP inherently reveals connection metadata to the provider |

## Known gaps and follow-ups

These are documented so operators can make informed decisions:

1. **Open registration.** Anyone who can reach the server can create an account.
   Add an invite code or admin approval before public exposure.
2. **Unthrottled relay.** Add per-user rate limits, quotas, and monitoring before
   enabling the relay for untrusted users.
3. **OAuth covers IMAP only.** SMTP XOAUTH2 remains an open follow-up.
4. **No server refresh tokens.** Server sessions expire; OAuth provider sessions refresh in the desktop app.
5. **No password reset or email verification.**
6. **Permissive CORS.** Restrict origins if the API is exposed directly.
7. **No CSP.** `tauri.conf.json` has `csp: null`; harden before treating the
   webview as a strict boundary.
8. **No mailbox encryption at rest.**
9. **No scheduled sync.** Sync is user-triggered.
10. **Redis unused.** It is deployed for future sessions/queues/realtime work.
11. **Shield is heuristic.** Detection is not a guarantee. External reputation
    checks depend on the user's provider keys and quotas.
12. **No end-to-end encryption yet.** Identity keys exist, but message signing,
    key exchange, and encrypted mail delivery are future work.
13. **External scanning reveals URLs.** VirusTotal and Google Safe Browsing
    receive the links extracted from opened messages unless external scanning is
    disabled.
14. **Attachment scanning is heuristic.** Local classification catches known
    dangerous types but is not a full antivirus. ClamAV must be installed and
    enabled separately, and VirusTotal uploads reveal the file to a third party
    even with consent.

## Security checklist for operators

Before exposing an Elyra server publicly:

- [ ] Use HTTPS with a valid certificate (Caddy profile or a reverse proxy).
- [ ] Set a unique `ELYRA_JWT_SECRET` of at least 32 random characters.
- [ ] Set strong, unique `POSTGRES_PASSWORD` and `REDIS_PASSWORD`.
- [ ] Keep PostgreSQL, Redis, and the API off the public network.
- [ ] Restrict the firewall to SSH, HTTP, and HTTPS.
- [ ] Publish SPF, DKIM, and DMARC for the relay sending domain.
- [ ] Use a dedicated relay mailbox or transactional provider.
- [ ] Add registration and relay rate limiting before public sign-ups.
- [ ] Back up PostgreSQL and test restores.
- [ ] Monitor API logs and relay failures.
- [ ] Decide whether open registration is acceptable for your audience.
- [ ] Tell users that relayed mail transits the server's SMTP provider.

## Security checklist for users

- [ ] Download only from the official releases page and verify the checksum.
- [ ] Use app-specific passwords, not your main provider password.
- [ ] Enable 2FA at the mail provider.
- [ ] Keep Windows and Elyra updated.
- [ ] Use full-disk encryption if the device may be lost.
- [ ] Treat JSON exports and database backups as sensitive.
- [ ] Sign out of Elyra Cloud on shared machines.
- [ ] Remove unused mail accounts so their keychain entries are deleted.

## Incident response

If you suspect a compromise:

1. **Mail account:** revoke the app password at the provider, remove the account
   in Elyra, and create a new app password.
2. **Elyra Cloud account:** change the password on the server, sign out on all
   devices, and rotate `ELYRA_JWT_SECRET` to invalidate all tokens.
3. **Server:** rotate `ELYRA_JWT_SECRET`, database and Redis passwords, and relay
   SMTP credentials; review API logs; restore from a known-good backup if data
   integrity is in doubt.
4. **Device:** disconnect from the network, run endpoint protection, rotate every
   credential stored in the keychain, and rebuild if malware is confirmed.
5. **Relay abuse:** disable `ELYRA_SMTP_HOST`, rotate relay credentials, and
   review provider logs.

Because there is no token revocation list, rotating the JWT secret is the
fastest way to invalidate all sessions.

## Reporting a vulnerability

Do not open a public issue for a security vulnerability. Contact the maintainer
privately through the repository owner's GitHub profile and include:

- Affected version and component.
- Reproduction steps or proof of concept.
- Impact assessment.
- Whether the issue is already public.

Please allow time for a fix before disclosure.

## Related pages

- [Architecture](architecture/ARCHITECTURE.md) — components and trust boundaries.
- [Server and API](server.md) — authentication and endpoint behavior.
- [Deployment](deployment.md) — production hardening steps.
- [`docs/security/ARCHITECTURE.md`](security/ARCHITECTURE.md) — short directory
  scope note.
- [`docs/security/e2ee.md`](security/e2ee.md) — planned end-to-end encryption
  protocol, threat model, key lifecycle, and limitations.
- [`docs/crypto/README.md`](crypto/README.md) — crypto layer notes.
