# Authenticated WebSocket live updates

- Status: open
- Labels: enhancement, server, desktop
- Area: `server/src/api.rs`, `apps/desktop/src/main.js`, docs

## Problem

The desktop client polls local SQLite and the REST API. Server notifications
and sync cursors change without pushing updates to open clients.

## Proposed work

- Add an authenticated WebSocket route using the existing JWT verifier.
- Subscribe by user id; send notification, sync-cursor, and mail-status events.
- Reconnect with exponential backoff and keep REST polling as a fallback.
- Document the event envelope in EN/UA server and command references.

## Acceptance criteria

- [ ] A signed-in client receives a notification within one second of creation.
- [ ] Expired tokens close the stream and restore the signed-out UI state.
- [ ] Polling fallback works when WebSocket is unavailable.

## Non-goals

- No mailbox-content streaming; only metadata events.
