# Tauri command reference

This page lists every command registered in
`apps/desktop/src-tauri/src/lib.rs` for Elyra 0.3.0. The webview can invoke only
these commands. Anything else is rejected by Tauri.

## How commands are called

The frontend uses the wrapper in `apps/desktop/src/services/tauri.js`:

```js
import { invoke } from "./services/tauri.js";

const page = await invoke("list_mailbox", {
  filter: "inbox",
  quickFilter: "all",
  query: "",
  limit: 100,
  offset: 0,
});
```

Rules that apply to every command:

- **Argument names are camelCase** in JavaScript even when the Rust parameter is
  `snake_case`. Tauri converts them automatically.
- **Optional arguments** are omitted or passed as `null`. Rust `Option<T>`
  parameters have the defaults documented below.
- **Return values** are JSON-serialized Rust structs. Structs with
  `#[serde(rename_all = "camelCase")]` return camelCase fields; the rest return
  `snake_case` fields. Both are noted per command.
- **Errors** are plain strings. `invoke` rejects with the message, which the UI
  shows in a toast or inline error.
- **Blocking network work** is marked `#[tauri::command(async)]` in Rust so it
  runs off the UI thread. From JavaScript it is still a normal promise.
- **Secrets never cross the boundary.** No command returns a cloud access token
  or a mail password.

The browser preview in `services/tauri.js` implements a subset of these commands
with `localStorage`. Desktop builds always call the Rust implementations.

## Command index

| Module | Commands |
| --- | --- |
| [Storage](#storage) | `initialize_local_storage`, `cache_email`, `list_cached_emails`, `set_local_setting`, `get_local_settings`, `set_local_settings`, `delete_local_setting` |
| [Mailbox](#mailbox) | `mailbox_stats`, `seed_demo_mailbox`, `reset_demo_mailbox`, `refresh_mailbox`, `list_mailbox`, `get_message`, `update_message`, `delete_message`, `bulk_update_messages`, `mark_all_read`, `empty_trash`, `save_draft`, `delete_draft`, `search_suggestions`, `export_mailbox` |
| [Providers](#providers) | `provider_presets`, `list_email_accounts`, `save_email_account`, `delete_email_account`, `test_email_account`, `test_email_settings`, `sync_email_account`, `sync_all_accounts` |
| [Sending](#sending) | `send_email`, `list_sent_emails` |
| [Security](#security) | `get_security_status`, `list_audit_events` |
| [Crypto](#crypto) | `generate_encryption_key`, `encrypt_text`, `decrypt_text`, `fingerprint` |
| [Diagnostics](#diagnostics) | `get_changelog`, `get_app_diagnostics` |
| [Cloud](#cloud) | `cloud_register`, `cloud_login`, `cloud_session`, `cloud_logout`, `cloud_list_notifications`, `cloud_send_notification`, `cloud_mark_notification_read`, `cloud_sync_cursor`, `cloud_update_cursor`, `cloud_health`, `cloud_api_url`, `cloud_set_api_url`, `cloud_mail_status` |

`secrets.rs` contains helper functions only; it registers no commands.

## Storage

Source: `apps/desktop/src-tauri/src/storage.rs`.

### `initialize_local_storage`

```ts
() => Promise<void>
```

Creates the app data directory, opens SQLite, applies pragmas, and creates the
base tables. Startup already calls this through `storage::initialize`, so the UI
normally does not need it.

### `cache_email`

```ts
(accountId: number, providerId: string, sender: string, subject: string,
 bodyText: string, folder: string, receivedAt: string) => Promise<void>
```

Upserts one cached message by `(account_id, provider_id)`. The folder must
already exist. Used by non-IMAP caching paths; IMAP sync uses its own richer
upsert.

### `list_cached_emails`

```ts
() => Promise<CachedEmail[]>
```

Returns cached rows in `snake_case`:

```json
[
  {
    "id": 1,
    "provider_id": "imap-42",
    "sender": "Ada <ada@example.com>",
    "subject": "Hello",
    "body_text": "Plain text body",
    "folder": "Inbox",
    "is_read": false,
    "is_starred": false
  }
]
```

### `set_local_setting`

```ts
(key: string, value: string) => Promise<void>
```

Upserts one `settings` row. Values are stored as strings. Prefer
`set_local_settings` for typed values.

### `get_local_settings`

```ts
() => Promise<Record<string, unknown>>
```

Returns all `settings` rows except keys starting with `cloud_`. Values are
parsed as JSON when possible and returned as strings otherwise.

### `set_local_settings`

```ts
(values: Record<string, unknown>) => Promise<void>
```

Upserts each entry. Keys starting with `cloud_` are ignored. Objects and arrays
are stored as JSON text.

### `delete_local_setting`

```ts
(key: string) => Promise<void>
```

Deletes one `settings` row. Does not delete keychain entries.

## Mailbox

Source: `apps/desktop/src-tauri/src/mailbox.rs`.

### `mailbox_stats`

```ts
() => Promise<MailboxStats>
```

Returns mailbox counters in `snake_case`:

```json
{
  "total": 42,
  "unread": 7,
  "starred": 3,
  "attachments": 5,
  "trash": 2,
  "spam": 1,
  "drafts": 4,
  "snoozed": 0,
  "accounts": 2,
  "storage_bytes": 1048576,
  "last_sync_at": "2026-11-16 10:00:00",
  "folder_counts": { "Inbox": 20, "Sent": 8 },
  "folder_unread": { "Inbox": 5 }
}
```

### `seed_demo_mailbox`

```ts
(force?: boolean) => Promise<MailboxStats>
```

Seeds the demo mailbox when it is empty. With `force: true`, deletes all
attachments and emails first. Default `false`.

### `reset_demo_mailbox`

```ts
() => Promise<MailboxStats>
```

Equivalent to `seed_demo_mailbox(true)`. Destroys all local messages, including
IMAP-synced mail, drafts, and sent records.

### `refresh_mailbox`

```ts
() => Promise<MailboxStats>
```

Updates the local `last_sync_at` setting and returns fresh statistics. It does
**not** contact IMAP.

### `list_mailbox`

```ts
(filter?: string, quickFilter?: string, query?: string,
 limit?: number, offset?: number) => Promise<MailboxPage>
```

Defaults: `filter = "inbox"`, `quickFilter = "all"`, `query = ""`,
`limit = 100` (clamped 1–500), `offset = 0`.

`filter` accepts `inbox`, `starred`, `snoozed`, `sent`, `drafts`, `archive`,
`spam`, `trash`, and the label views `important`, `updates`, `security`,
`personal`. Unknown values are treated as a folder/label lookup.

`quickFilter` accepts `all`, `unread`, `starred`, `attachments`.

Returns:

```json
{
  "messages": [
    {
      "id": 12,
      "sender": "Ada Lovelace",
      "initials": "AL",
      "subject": "Notes",
      "preview": "First lines of the body",
      "time": "2h",
      "received_at": "2026-11-16 08:00:00",
      "folder": "Inbox",
      "tag": "Important",
      "color": "coral",
      "is_read": false,
      "is_starred": false,
      "has_attachments": true,
      "snoozed_until": null
    }
  ],
  "total": 1,
  "unread": 1,
  "starred": 0,
  "has_more": false,
  "filter": "inbox",
  "quick_filter": "all"
}
```

`MessageSummary` has no serde rename, so its fields are `snake_case`.

### `get_message`

```ts
(id: number) => Promise<MessageDetail>
```

Returns the full message and marks it read. Shape:

```json
{
  "message": { "...": "MessageSummary" },
  "recipient": "you@example.com",
  "body_text": "Plain text body",
  "body_html": null,
  "attachments": [
    { "id": 3, "name": "report.pdf", "mime_type": "application/pdf", "size_bytes": 42000 }
  ]
}
```

`body_html` is available in the database but the reader displays escaped plain
text, not raw HTML.

### `update_message`

```ts
(id: number, isRead?: boolean, isStarred?: boolean, folder?: string) => Promise<void>
```

Updates only the fields that are provided. `folder` is a folder name such as
`Archive`, `Trash`, `Spam`, `Snoozed`, or `Inbox`. With no fields, it is a no-op.

### `delete_message`

```ts
(id: number, permanent?: boolean) => Promise<void>
```

Soft-deletes by default (sets `deleted_at`). With `permanent: true`, deletes the
row and its attachments. Permanent deletion cannot be undone.

### `bulk_update_messages`

```ts
(ids: number[], action: string, until?: string) => Promise<number>
```

Returns the number of changed rows. Maximum 500 ids per call; an empty list
returns `0`.

| Action | Effect |
| --- | --- |
| `read` | Sets `is_read = 1` |
| `unread` | Sets `is_read = 0` |
| `star` | Sets `is_starred = 1` |
| `unstar` | Sets `is_starred = 0` |
| `trash` | Moves to **Trash** |
| `archive` | Moves to **Archive** |
| `spam` | Moves to **Spam** |
| `inbox` / `restore` | Moves to **Inbox** |
| `snooze` | Moves to **Snoozed** and sets `snoozed_until` (default now + 1 day) |
| `delete` | Permanently deletes the rows |

Unknown actions return `Unsupported action: <name>`.

### `mark_all_read`

```ts
(filter?: string) => Promise<number>
```

Marks every message matching the current mailbox view read and returns the
count. Default `filter = "inbox"`.

### `empty_trash`

```ts
() => Promise<number>
```

Permanently deletes every message in **Trash** and returns the count.

### `save_draft`

```ts
(id?: number, recipient: string, subject: string, body: string) => Promise<number>
```

Updates an existing draft when `id` belongs to the **Drafts** folder; otherwise
inserts a new draft. Returns the draft id.

### `delete_draft`

```ts
(id: number) => Promise<boolean>
```

Deletes the row only if it is in **Drafts**. Returns whether a row was deleted.

### `search_suggestions`

```ts
(query: string, limit?: number) => Promise<string[]>
```

Returns up to `limit` suggestions (default 6, clamped 1–20) derived from
matching senders and subjects.

### `export_mailbox`

```ts
() => Promise<unknown>
```

Returns a JSON object containing folders, messages, and metadata for all
non-deleted messages. The object can be large and contains message bodies.
Treat the export as sensitive.

## Providers

Source: `apps/desktop/src-tauri/src/providers.rs`.

### `provider_presets`

```ts
() => Promise<ProviderPreset[]>
```

Returns the built-in presets in `camelCase`:

```json
[
  {
    "id": "gmail",
    "label": "Gmail",
    "imapHost": "imap.gmail.com",
    "imapPort": 993,
    "imapSecurity": "tls",
    "smtpHost": "smtp.gmail.com",
    "smtpPort": 465,
    "smtpSecurity": "tls",
    "hint": "Use an app password"
  }
]
```

Preset ids: `gmail`, `outlook`, `icloud`, `yahoo`, `fastmail`, `proton`,
`custom`.

### `list_email_accounts`

```ts
() => Promise<EmailAccount[]>
```

Returns account metadata in `camelCase`:

```json
[
  {
    "id": 1,
    "provider": "gmail",
    "email": "you@gmail.com",
    "displayName": "Personal",
    "username": "you@gmail.com",
    "imapHost": "imap.gmail.com",
    "imapPort": 993,
    "imapSecurity": "tls",
    "smtpHost": "smtp.gmail.com",
    "smtpPort": 465,
    "smtpSecurity": "tls",
    "hasPassword": true,
    "lastSyncAt": "2026-11-16 10:00:00",
    "syncError": null
  }
]
```

`hasPassword` reports keychain presence; the password itself is never returned.

### `save_email_account`

```ts
(account: EmailAccountInput) => Promise<EmailAccount>
```

Creates or updates an account. The input uses `camelCase`:

```json
{
  "id": null,
  "provider": "custom",
  "email": "you@example.com",
  "displayName": "Work",
  "username": "you@example.com",
  "password": "app-password",
  "imapHost": "imap.example.com",
  "imapPort": 993,
  "imapSecurity": "tls",
  "smtpHost": "smtp.example.com",
  "smtpPort": 587,
  "smtpSecurity": "starttls"
}
```

Behavior:

- Email is trimmed and lowercased; it must contain `@` and be at least 3
  characters.
- Empty `provider` becomes `custom`.
- `imapPort`/`smtpPort` default to 993 and 465 when omitted.
- Security values other than `tls`/`starttls` normalize to `starttls`.
- A non-empty password is written to the keychain under `imap-account-<id>`.
- A blank password on update keeps the existing keychain value.
- Returns the stored account metadata.

### `delete_email_account`

```ts
(id: number) => Promise<void>
```

Deletes the account row, its cached messages (cascade), and its keychain entry.

### `test_email_account`

```ts
(id: number) => Promise<AccountTest>
```

Tests a saved account using its keychain password. Fails if no password is
stored. Returns:

```json
{ "imapOk": true, "smtpOk": true, "detail": "IMAP inbox reachable (128 messages); SMTP ready" }
```

### `test_email_settings`

```ts
(account: EmailAccountInput) => Promise<AccountTest>
```

Tests unsaved settings. If `password` is blank and `id` is provided, the stored
keychain password is used. Otherwise a password is required. Nothing is saved.

### `sync_email_account`

```ts
(accountId: number, limit?: number) => Promise<SyncAccountResult>
```

Fetches the newest `limit` messages from **INBOX** (default 40, clamped 1–200)
and upserts them. Returns:

```json
{ "accountId": 1, "email": "you@gmail.com", "imported": 12, "error": null }
```

A sync failure is recorded in `syncError` and returned in `error`; the command
itself resolves so the UI can show per-account state.

### `sync_all_accounts`

```ts
(limit?: number) => Promise<SyncSummary>
```

Runs the same sync for every account. Returns:

```json
{
  "accounts": [
    { "accountId": 1, "email": "you@gmail.com", "imported": 12, "error": null }
  ],
  "imported": 12
}
```

Only INBOX is synced. There is no scheduled background sync in 0.3.0.

## Sending

Source: `apps/desktop/services/sendemail.rs`.

### `send_email`

```ts
(to: string, subject: string, body: string, accountId?: number) => Promise<void>
```

Sends a plain-text message and records it in **Sent** on success. Order:

1. If `accountId` is provided, use that account's SMTP configuration and
   keychain password.
2. Otherwise, try the complete `ELYRA_SMTP_*` environment configuration.
3. Otherwise, if signed in to Elyra Cloud, use the server relay.

A failure rejects with a combined error that includes the environment problem
and the relay problem. Failed sends are not recorded.

### `list_sent_emails`

```ts
() => Promise<SendEmail[]>
```

Returns sent records in `snake_case`:

```json
[{ "to": "friend@example.com", "subject": "Hello", "body": "Plain text body" }]
```

## Security

Source: `apps/desktop/src-tauri/src/security.rs`.

### `get_security_status`

```ts
() => Promise<SecurityStatus>
```

Runs the SQLite posture check and records an audit event. Returns `snake_case`
fields:

```json
{
  "status": "protected",
  "checks_passed": 4,
  "checks_total": 4,
  "foreign_keys": true,
  "journal_mode": "wal",
  "secure_delete": true,
  "audit_events": 12
}
```

`status` is `protected` only when the checks pass. The UI must display the real
result and must not substitute a fake pass.

### `list_audit_events`

```ts
(limit?: number) => Promise<AuditEvent[]>
```

Returns newest audit rows first. Default 20, clamped 1–100.

```json
[
  {
    "id": 12,
    "created_at": "2026-11-16 10:00:00",
    "event_type": "security.check",
    "outcome": "success",
    "details": "SQLite security policy check completed"
  }
]
```

`outcome` is one of `success`, `failure`, `warning`. The table is capped at 500
rows.

## Crypto

Source: `apps/desktop/src-tauri/src/crypto.rs`.

### `generate_encryption_key`

```ts
() => Promise<string>
```

Returns a base64-encoded 32-byte key from OS randomness. The Security Lab keeps
it in memory only.

### `encrypt_text`

```ts
(key: string, plaintext: string) => Promise<string>
```

Encrypts with AES-256-GCM using a fresh 96-bit nonce. Returns base64 of
`nonce || ciphertext || tag`.

### `decrypt_text`

```ts
(key: string, payload: string) => Promise<string>
```

Validates the payload and authentication tag. Rejects invalid base64, short
payloads, wrong keys, tampered ciphertext, and non-UTF-8 plaintext.

### `fingerprint`

```ts
(value: string) => Promise<string>
```

Returns a colon-separated SHA-256 fingerprint for display and comparison only.
It is not an authentication mechanism.

## Diagnostics

Source: `apps/desktop/src-tauri/src/diagnostics.rs`.

### `get_changelog`

```ts
() => Promise<ChangeEntry[]>
```

Returns the static release notes rendered in the **Changes** tab:

```json
[
  {
    "version": "0.3.0",
    "date": "2026-11-14",
    "title": "OS keychain secrets",
    "detail": "Elyra Cloud sessions and IMAP passwords are now stored in the Windows Credential Manager...",
    "status": "ready"
  }
]
```

`status` is `shipped`, `ready`, or `next`. The authoritative log is
[`changes.md`](../../changes.md).

### `get_app_diagnostics`

```ts
() => Promise<AppDiagnostics>
```

Returns `snake_case` diagnostics:

```json
{
  "app_version": "0.3.0",
  "engine": "Rust + Tauri + SQLite",
  "os": "windows",
  "arch": "x86_64",
  "sqlite_version": "3.46.0",
  "database_path": "C:\\Users\\you\\AppData\\Local\\com.elyra.desktop\\elyra.sqlite3",
  "database_bytes": 1048576,
  "mailbox": { "...": "MailboxStats" },
  "cloud_configured": true,
  "keychain_available": true
}
```

`cloud_configured` is true when the resolved API base is not the local default.
`keychain_available` reports whether the OS credential store is reachable.

## Cloud

Source: `apps/desktop/src-tauri/src/cloud.rs`. All network commands are
`#[tauri::command(async)]`.

### `cloud_register`

```ts
(email: string, password: string, displayName: string) => Promise<CloudSessionView>
```

Calls `POST {api}/v1/auth/register`, stores the session in the keychain under
`cloud_session`, and returns the session view:

```json
{
  "expiresAt": "2026-11-17T12:00:00Z",
  "user": {
    "id": "0f3f6c1e-...",
    "email": "you@example.com",
    "displayName": "You",
    "createdAt": "2026-11-16T12:00:00Z"
  },
  "signedIn": true
}
```

The access token is **not** returned.

### `cloud_login`

```ts
(email: string, password: string) => Promise<CloudSessionView>
```

Calls `POST {api}/v1/auth/login`, stores the session, and returns the same view.

### `cloud_session`

```ts
() => Promise<CloudSessionView | null>
```

Loads the stored session. Returns `null` when signed out. Migrates a legacy
`settings.cloud_session` row to the keychain on first read.

### `cloud_logout`

```ts
() => Promise<void>
```

Deletes the keychain `cloud_session` entry and any legacy SQLite session row. It
does not delete the server account, local mail, or mail passwords.

### `cloud_list_notifications`

```ts
() => Promise<unknown>
```

Calls `GET {api}/v1/notifications` with the bearer token and returns the server
JSON array (up to 100, newest first). Fails when signed out.

### `cloud_send_notification`

```ts
(recipientEmail: string, title: string, body: string) => Promise<unknown>
```

Calls `POST {api}/v1/notifications`. The recipient must already have an Elyra
account. Returns the created notification JSON.

### `cloud_mark_notification_read`

```ts
(id: string) => Promise<void>
```

Calls `PATCH {api}/v1/notifications/{id}/read`. Only the recipient can mark a
notification read; other ids return `404`.

### `cloud_sync_cursor`

```ts
() => Promise<unknown>
```

Calls `GET {api}/v1/sync/cursor` and returns `{ cursor, synced_at }`. The cursor
is opaque; 0.3.0 does not run a scheduled sync loop.

### `cloud_update_cursor`

```ts
(cursor?: string | null) => Promise<void>
```

Calls `PUT {api}/v1/sync/cursor` with `{ cursor }`. `null` clears the cursor.

### `cloud_health`

```ts
() => Promise<unknown>
```

Calls `GET {api}/health` without authentication. Returns
`{ status, service }`.

### `cloud_api_url`

```ts
() => Promise<string>
```

Returns the resolved API base (no trailing slash) using the documented order:
`ELYRA_API_URL` → `settings.api_url` → `ELYRA_DEFAULT_API_URL` →
`http://127.0.0.1:8787`.

### `cloud_set_api_url`

```ts
(url: string) => Promise<string>
```

Validates and saves a server URL in `settings.api_url`, or clears it when the
input is empty. The URL must start with `http://` or `https://`. Returns the
effective base after saving.

### `cloud_mail_status`

```ts
() => Promise<unknown>
```

Calls `GET {api}/v1/mail/status` and returns
`{ configured: boolean, from: string | null }`. Requires a signed-in session.

## Related pages

- [Architecture](../architecture.md) — module responsibilities.
- [Server and API](../server.md) — the routes behind the cloud commands.
- [User guide](../user-guide.md) — how the UI uses these commands.
- [Development](../development.md) — adding and registering a command.
