# Local issue files

GitHub CLI/token access is not available in every environment, so this directory
mirrors issue tracking inside the repository. Files under `issues/closed/` are
completed work; files under `issues/open/` are follow-ups. GitHub templates live
in [`.github/ISSUE_TEMPLATE/`](../.github/ISSUE_TEMPLATE/).

## Closed

- [001 — Provider OAuth with PKCE](closed/001-provider-oauth-pkce.md)
- [002 — Concrete Rust/API correctness fixes](closed/002-rust-and-api-bug-fixes.md)
- [007 — Dependency security alerts](closed/007-dependency-security-alerts.md)

## Open

- [003 — SMTP XOAUTH2 for OAuth-connected accounts](open/003-oauth-smtp-xoauth2.md)
- [004 — Authenticated WebSocket live updates](open/004-websocket-live-updates.md)
- [005 — Full bilingual link and anchor audit](open/005-full-bilingual-link-audit.md)
- [006 — OAuth hardening follow-ups](open/006-oauth-hardening.md)
- [008 — Migrate off the IMAP alpha dependency](open/008-imap-stable-migration.md)

When an issue is finished, move the file to `issues/closed/`, set `Status: done`,
and record validation in [`changes.md`](../changes.md).
