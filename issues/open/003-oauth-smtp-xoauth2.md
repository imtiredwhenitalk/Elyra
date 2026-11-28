# SMTP XOAUTH2 for OAuth-connected accounts

- Status: open
- Labels: enhancement, oauth, desktop, smtp
- Area: `apps/desktop/src-tauri/src/providers.rs`, `services/sendemail.rs`

## Problem

Provider OAuth currently secures IMAP sync only. `providers::smtp_config`
rejects `auth_method = "oauth2"`, and the compose account picker hides those
accounts from direct SMTP sending. Users must keep an app password or use the
Elyra Cloud relay.

## Proposed work

- Add an OAuth XOAUTH2 path to the `lettre` SMTP transport, including
  `AUTHENTICATE XOAUTH2` and token refresh through `oauth::access_token`.
- Add the Microsoft `SMTP.Send` and verify Google's allowed SMTP scope.
- Extend `test_email_settings` so SMTP reports OAuth readiness instead of a
  static failure.
- Update EN/UA user guides, command reference, and limitations once supported.

## Acceptance criteria

- [ ] Sending through Gmail and Outlook OAuth accounts succeeds without an app
      password.
- [ ] Token refresh and revocation errors are surfaced without exposing tokens.
- [ ] Browser preview behavior remains clearly documented as unsupported.

## Non-goals

- No Elyra Cloud provider OAuth; the cloud still uses Argon2 + JWT.
- No automatic fallback to the server relay without user consent.
