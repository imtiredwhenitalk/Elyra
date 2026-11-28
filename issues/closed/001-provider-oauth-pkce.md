# Provider OAuth with PKCE

- Status: done
- Labels: enhancement, auth, desktop, docs
- Area: `apps/desktop/src-tauri/src/oauth.rs`, `providers.rs`, desktop UI

## Problem

Elyra only supported IMAP app passwords for Gmail and Outlook. The roadmap
listed provider OAuth with PKCE as `[NEXT]` and offered no reconnect or revoke
path.

## Delivered

- Authorization Code + PKCE (`S256`) native desktop flow with random `state`
  and loopback redirect `http://127.0.0.1:<random-port>/callback`.
- `oauth_providers`, `oauth_start`, `oauth_status`, and `oauth_revoke` Tauri
  commands. No access, refresh, or client secret crosses the webview.
- Client IDs configured through `ELYRA_GOOGLE_CLIENT_ID` and
  `ELYRA_MICROSOFT_CLIENT_ID` at build or runtime.
- Token JSON in the OS keychain under `oauth-account-<id>`; SQLite keeps only
  `auth_method` and email metadata.
- IMAP `AUTHENTICATE XOAUTH2` for connected accounts, automatic token refresh,
  and keychain cleanup on account deletion.
- Account modal buttons for Gmail/Outlook, OAuth status, and disconnect.
- English/Ukrainian documentation: `docs/en/oauth.md`, `docs/ua/oauth.md`,
  indexes, guides, configuration, command reference, and security pages.

## Validation

- RFC 7636 PKCE vector, random-token, provider-alias, authorization URL, and
  loopback state tests added in `oauth.rs`.
- `cargo fmt` applied to touched Rust files; `node --check` passed for the
  desktop JavaScript.
- Full local Rust compilation remains blocked by the missing MSVC `link.exe`
  in this environment; CI is the build source of truth.

## Post-commit audit fixes

- Google revocation resolves canonical provider ids from the token set, posts
  the refresh token when available, and checks the response status.
- OAuth UI/commands require a usable OS keychain, so non-Windows builds do not
  start a flow that cannot store tokens.
- `connected` is false for expired sessions without a refresh token.
- IMAP XOAUTH2 uses the stored account email, not the editable password
  username; conversion from a password account clears the stale password.
- Token endpoint 400/401 errors include a reconnect hint.

## Follow-ups

- `issues/open/003-oauth-smtp-xoauth2.md`
- `issues/open/004-websocket-live-updates.md`
- `issues/open/006-oauth-hardening.md`
