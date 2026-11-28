# Concrete Rust/API correctness fixes

- Status: done
- Labels: bug, server, desktop
- Area: server auth, provider testing, cloud session, integrations, outbox

## Problems fixed

1. **Case-sensitive email identity.** Registration/login used exact-case matching
   while notification recipients used `lower(email)`, allowing duplicate
   accounts and ambiguous delivery. Emails are now trimmed and lowercased at the
   API boundary; lookup and a new unique index use `lower(email)`.
2. **SMTP test false positives.** `smtp_reachable` only opened a TCP socket and
   performed a TLS handshake for implicit TLS. `test_settings` now exercises the
   real `lettre` transport (`EHLO`, STARTTLS, NOOP) through `test_connection()`.
3. **Duplicate outbox delivery.** Concurrent async flushes could both read the
   same queued row before deleting it. `flush_outgoing_queue` is now serialized
   with a process-wide mutex around the read-send-delete sequence.
4. **Expired cloud sessions shown as signed in.** `CloudSessionView` hardcoded
   `signed_in: true`. Expiry is now parsed from RFC 3339 and stale keychain and
   legacy SQLite sessions are discarded.
5. **Rejected integration keys persisted.** `connect_integration` stored the key
   before validation. It now validates first and leaves any previously valid
   key untouched when validation fails.
6. **Case-sensitive Bearer scheme.** `bearer_token` rejected RFC-compliant
   schemes such as `bearer` and `BEARER`; parsing is now case-insensitive.

## Post-commit audit fixes

- Server migration detects duplicate normalized emails before normalizing rows
  and creating the `lower(email)` unique index, with an actionable conflict
  message for legacy databases.
- Failed integration validation no longer deletes a previously valid key.
- Expired cloud sessions are discarded even when keychain deletion fails.
- Poisoned outbox locks recover instead of permanently failing later flushes.

## Validation

- Unit tests added for normalized email identity, lowercase Bearer scheme, and
  expired/malformed cloud sessions.
- Existing Rust tests cover credential validation and API status mapping.
- Full local Rust compilation remains blocked by missing MSVC `link.exe`;
  `cargo fmt` and source review completed locally.
