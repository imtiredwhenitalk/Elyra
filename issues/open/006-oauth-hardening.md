# OAuth hardening follow-ups

- Status: open
- Labels: enhancement, oauth, security
- Area: `apps/desktop/src-tauri/src/oauth.rs`, `providers.rs`, `secrets.rs`

## Problem

The post-commit audit found remaining edge cases that are safe to defer but
should be tracked explicitly:

1. **Loopback callback DoS.** `receive_code` aborts the whole sign-in on the
   first callback with a mismatched `state`. The port is ephemeral, but a local
   process can still force a retry.
2. **Provider identity verification.** `oauth_start` trusts the user-entered
   address instead of validating it against an ID token or provider profile.
   IMAP now uses the stored account email, so editing `username` no longer
   breaks XOAUTH2, but a wrong address entered at sign-in is still possible.
3. **Linux keychain.** OAuth is correctly hidden where the OS keychain is
   unavailable. A cross-platform secret backend would be required to offer it
   on Linux.

## Proposed work

- Ignore callbacks with a mismatched `state` and keep waiting until the original
  deadline, while still responding with `400 Bad Request`.
- Request and verify `id_token` (Google) or `/me` (Microsoft) and require the
  returned address to match the account before storing tokens.
- Evaluate a supported Linux secret-service backend before enabling OAuth there.

## Acceptance criteria

- [ ] A stray loopback request with bad `state` cannot terminate a valid flow.
- [ ] A token set is never stored for an email the provider did not authorize.
- [ ] Documentation states exactly which platforms offer OAuth.

## Non-goals

- No SMTP XOAUTH2 here; that remains `issues/open/003-oauth-smtp-xoauth2.md`.
- No change to Elyra Cloud authentication (Argon2 + JWT).
