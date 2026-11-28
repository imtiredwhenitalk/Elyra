# Migrate off the IMAP alpha dependency

- Status: open
- Labels: dependencies, security, follow-up
- Area: `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/src/providers.rs`

## Problem

The only stable `imap` release (2.4.1) depends on `nom` 5, which enables the
vulnerable `lexical-core` 0.7 parser. To clear the Dependabot alert without a
full IMAP client rewrite, Elyra now uses `imap` 3.0.0-alpha.15. That version
uses `nom` 7 and a boxed `imap::Connection`, but it is still a pre-release.

## Proposed work

- Track a stable `imap` 3.x release, or evaluate a maintained async client such
  as `imap-client`/`imap-next`.
- Replace `ClientBuilder`/`imap::Connection` usage only when a stable API is
  available; do not downgrade to 2.4 and reintroduce `lexical-core` 0.7.
- Re-run IMAP OAuth and password sync checks after the migration.

## Acceptance criteria

- [ ] No pre-release IMAP client remains in the workspace lockfile.
- [ ] `lexical-core` 0.7 and `nom` 5 are absent.
- [ ] IMAP XOAUTH2, STARTTLS, and implicit TLS account tests still pass.

## Non-goals

- No SMTP XOAUTH2 change; that remains `issues/open/003-oauth-smtp-xoauth2.md`.
