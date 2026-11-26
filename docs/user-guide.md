# User guide

This guide describes the Elyra 0.4.0 desktop interface and the behavior behind
each control. Menu labels are shown in **bold**.

## Interface overview

Elyra uses a three-column desktop layout plus a right-hand context rail.

| Area | What it contains |
| --- | --- |
| Left sidebar | Compose button, mailbox navigation, Spaces, account card, Changes, Settings |
| Center top bar | Greeting, search, quick filter, mailbox title and actions, select-all |
| Center list | Message rows with sender, subject, preview, tag, time, star, and selection |
| Center reader | Full message view that replaces the list when a message is opened |
| Right rail | Mailbox statistics, local storage meter, connected accounts, notification and connect buttons |
| Overlays | Compose modal, account modal, auth modal, command palette, notification popover, toasts |

The window is responsive. Narrow widths collapse the right rail and adapt the
reader. `prefers-reduced-motion` and the **Reduce motion** setting disable
non-essential animation.

## Mailboxes and Spaces

### System mailboxes

| Mailbox | Contents |
| --- | --- |
| **Inbox** | Messages delivered to the inbox folder |
| **Starred** | Any message with the starred flag |
| **Snoozed** | Messages moved to the snoozed folder |
| **Sent** | Messages recorded after a successful send |
| **Drafts** | Locally saved drafts |
| **Archive** | Messages moved out of the inbox |
| **Spam** | Messages marked as spam |
| **Trash** | Messages waiting before permanent deletion |

### Spaces

**Important**, **Updates**, and **Security** are label-style views backed by the
same local folders. They are useful for separating decisions, product activity,
and account alerts. The seeded demo mailbox includes examples of each.

### Quick filter

The filter control above the list switches the current mailbox between:

- **All** — no additional filter.
- **Unread** — only messages with `is_read = 0`.
- **Starred** — only starred messages.
- **Important** — only messages with `is_important = 1`.
- **High priority** — messages at `high` or `urgent` priority.
- **Urgent** — only messages at `urgent` priority.
- **Attachments** — only messages with at least one attachment.

The quick filter and the search query combine. Both reset when you switch
mailboxes or use **Reset view** from an empty state.

### Search

Type in the search field to search senders, subjects, and message bodies in the
local database. Search is debounced, so results update shortly after you stop
typing. Search suggestions are also available through the
`search_suggestions` command.

Search is local. It does not query the mail provider and does not send the query
to Elyra Cloud.

### Advanced search

Select **Advanced** next to the quick filter to open the local filter panel. You
can combine:

- **From**, **To**, and **Subject** text matching.
- **Folder**, **From date**, and **To date** (`YYYY-MM-DD`).
- Multiple **Priority**, **Security**, and **Reputation** values.
- **Has attachments** and **Important only** flags.

Select **Apply filters** to run the search through the Rust `advanced_search`
command, or **Clear filters** to return to the normal mailbox view. Advanced
filters reset when you change mailboxes. Everything is evaluated against SQLite
on this device; no filter value is sent to a server.

The **Sync status** badge in the header shows the current outbox count, a sync
error, or the last successful sync time.

### Unread counts

Mailbox counters show unread counts when available and fall back to total
counts. They refresh after reads, stars, moves, sends, syncs, and resets.

### Refresh

The refresh button in the header runs a local mailbox refresh and updates the
statistics. The **Refresh mailbox** action is also available in the command
palette. Refreshing does **not** contact IMAP; use **Sync** on a mail account for
that. The refresh button for accounts is described in
[Mail accounts](#mail-accounts).

## Reading messages

Click a message row to open the reader. The reader replaces the list and shows:

- Sender, recipient or reply-to, subject, tag, time, and a local-only badge.
- Importance and priority indicators. The reader shows an **Important** chip and
  a priority chip with the reason, plus a **Priority** selector and a
  **Mark important / Important** action.
- The plain-text body. HTML from providers is not rendered directly; message
  text is escaped before display.
- Attachment cards with name, detected type, size, a risk badge, the scan
  source, and any reasons. Each card has scan, consent-based VirusTotal, and
  guarded download controls.
- Toolbar actions: **Archive**, **Spam**, **Trash**, and **More**.
- Footer actions: **Star/Starred**, **Mark important/Important**,
  **Reply**, **Forward**, **Mark unread**, or **Edit draft** for drafts.

Use **Back to …** or press `Escape` to close the reader. Opening another mailbox
also closes it.

### Actions

| Action | Effect |
| --- | --- |
| Star | Toggles `is_starred`; the message appears in **Starred** |
| Mark important | Toggles `is_important`; the message appears in **Important** |
| Priority | Sets `low`, `normal`, `high`, or `urgent` and records a manual choice |
| Mark unread | Sets `is_read = 0` and returns to the list |
| Archive | Moves the message to **Archive** |
| Spam | Moves the message to **Spam** |
| Trash | Moves the message to **Trash** |
| Restore | Moves a trashed message back to **Inbox** |
| Delete | Permanently deletes the row after confirmation; in **Trash**, uses permanent deletion |
| Reply | Opens compose with `Re: <subject>` and the sender as recipient |
| Forward | Opens compose with `Fwd: <subject>` and a quoted plain-text body |
| Edit draft | Reopens the draft in the compose modal |

Actions are implemented by the Rust core. In browser preview they fall back to
`localStorage` so the interface remains demonstrable.

## Bulk actions

Select one or more messages with the row checkbox or the header select-all
control. A bulk bar appears with:

**Read**, **Unread**, **Star**, **Important**, **Not important**, **Archive**,
**Spam**, **Restore**, **Trash**, and **Delete**.

In **Trash**, only **Restore** and **Delete** are offered. Permanent deletion
asks for confirmation. Selection is cleared after a successful bulk action.

## Composing and sending

Click **Compose** or use the command palette. The modal contains:

- **From** — a sending account selector populated from accounts that have an
  SMTP host. If none exist, the selector is empty.
- **To** — recipient address.
- **Subject** — subject line.
- **Message body** — prefilled with your signature when one is configured.
- **Save draft** — stores the message in **Drafts**.
- **Delete draft** — appears when editing an existing draft.
- **Send** — submits the message.

### Send path

Elyra tries these paths in order:

1. **Selected account** — the account's SMTP host, port, security mode, username,
   and keychain password are used.
2. **Environment SMTP** — if no account is selected, `ELYRA_SMTP_FROM`,
   `ELYRA_SMTP_HOST`, `ELYRA_SMTP_USERNAME`, and `ELYRA_SMTP_PASSWORD` (plus
   optional `ELYRA_SMTP_PORT`) are used.
3. **Elyra Cloud relay** — if the environment is incomplete and you are signed
   in, the message is sent through the server's optional relay.

If all three fail, the send fails and the compose modal stays open with an error
toast. A successful send records the message in **Sent** with the sender name
and recipient. Failed sends are not recorded as sent.

The server relay sends from the operator's `ELYRA_SMTP_FROM` address and sets
`Reply-To` to your Elyra account email. The server does not persist the body.

### Drafts

Drafts are stored locally in SQLite. Saving a draft with no recipient or subject
uses safe placeholders. Opening a draft from **Drafts** restores the recipient,
subject, and body. Sending a draft deletes the draft row. Drafts are not synced
to the provider in 0.4.0.

## Mail accounts

Open **Settings → Mail accounts**, or click the account controls in the right
rail.

### Add an account

1. Click **Add account**.
2. Choose a provider preset:
   - Gmail
   - Outlook / Microsoft 365
   - iCloud Mail
   - Yahoo Mail
   - Fastmail
   - Proton Mail (Proton Mail Bridge)
   - Custom IMAP / SMTP
3. Enter the email address, display name, and username. The username defaults to
   the email address for most providers.
4. Enter the password or app password.
5. For **Custom**, enter IMAP and SMTP hosts, ports, and security. TLS is the
   default; STARTTLS is available.
6. Click **Test connection** to check IMAP and SMTP before saving.
7. Click **Save**.

Provider presets fill the standard hosts and ports. You can still edit them
after selecting a preset.

### Passwords and the keychain

Passwords are written to the Windows Credential Manager through `keyring-rs`
under the service `com.elyra.desktop`, using the entry
`imap-account-<account id>`. SQLite stores account metadata only: provider,
address, username, hosts, ports, security modes, sync timestamps, and errors.

Editing an account with a blank password keeps the stored password. Removing an
account deletes its keychain entry and its locally cached messages. The
**Diagnostics** panel reports whether the keychain is reachable.

### Test connection

**Test connection** opens IMAP, selects INBOX, reports the message count, and
checks the SMTP transport. Results are shown inline in the modal. A failed test
does not save the account.

### Sync

**Sync** pulls the newest messages from the provider's **INBOX** into the local
mailbox. The desktop requests 40 messages per account by default; the core
accepts 1–200. Sync behavior:

- Fetches UID, flags, internal date, and the full message with `BODY.PEEK[]` so
  messages are not marked read on the server.
- Parses headers and bodies with `mail-parser`.
- Upserts by `(account_id, provider_id)` so repeated syncs do not duplicate.
- Preserves local read and starred state across syncs.
- Records `last_sync_at` or `sync_error` per account.
- Only INBOX is synced. Sent, Drafts, Archive, Spam, and Trash are not pulled
  from the provider in 0.4.0.

**Sync all** runs the same operation for every configured account and reports
how many messages were imported.

### Edit and remove

Use the pencil button on an account row to edit it, the refresh button to sync
it, and the trash button to remove it. Removing an account asks for
confirmation because its local messages are deleted.

### Provider notes

- **Gmail** requires 2-Step Verification and an App Password. A normal account
  password will be rejected.
- **Outlook/Microsoft 365** may require an app password or tenant consent.
  OAuth is planned.
- **iCloud** requires an app-specific password.
- **Proton Mail** requires Proton Mail Bridge, which exposes local IMAP/SMTP on
  `127.0.0.1`. Use the Bridge credentials, not your Proton account password.
- **Custom** servers must support TLS or STARTTLS. Plaintext IMAP/SMTP is not
  supported.

## Elyra Cloud

Elyra Cloud is the optional account and notification service. It is not required
for the local mailbox.

### Register or sign in

Open **Settings → Elyra Cloud → Connect account**, or accept the first-run
onboarding prompt.

- **Create an account** sends your email, display name, and password to the
  configured server.
- **I already have an account** signs in.
- The server URL field accepts the API base, for example
  `https://mail.example.com/api`.
- Passwords must be at least 12 characters.
- **Continue offline** closes the onboarding prompt without creating an account.

The access token is stored in the keychain as `cloud_session`. The webview only
receives a session view: user id, email, display name, and expiry. It never
receives the raw token.

### Notifications

When signed in, the bell button opens the notification popover. It lists up to
100 notifications for your account. Clicking one marks it read. You can also
send an in-app notification to another registered Elyra user from the
notification composer. The recipient must already have an account.

Notifications are server-side records with a title and body. They are not email
messages and are not end-to-end encrypted.

### Server URL

The server URL is stored in SQLite `settings.api_url`. Change it under
**Settings → Elyra Cloud → Server URL → Save server**. The resolution order is:

1. `ELYRA_API_URL` environment variable.
2. The saved server URL.
3. `ELYRA_DEFAULT_API_URL` compiled into the installer.
4. `http://127.0.0.1:8787`.

The URL must start with `http://` or `https://`. Trailing slashes are stripped.
In production use HTTPS and include `/api` in the path.

### Sign out

**Sign out** deletes the keychain cloud session and clears the in-memory session.
It does not delete your server account, local mail, or mail-account passwords.

## Settings

### Identity

- **Display name** — shown in the sidebar, profile, and outgoing mail from the
  server relay.
- **Elyra domain handle** — a display-only prototype handle such as
  `alex@elyra.com`. It is not a real mailbox and cannot receive mail in 0.4.0.
- **Signature** — prefilled into new compose windows.

### Appearance

| Setting | Options |
| --- | --- |
| Theme | Warm light, Forest dark, Midnight blue, OLED black |
| Accent | Moss, Ocean, Ember, Violet |
| Density | Comfortable, Compact |
| Reduce motion | Disables non-essential animation |

Theme, accent, density, and motion are applied immediately and persisted. The
OLED theme uses true black surfaces for OLED displays.

### Behavior

- **Local-only preview** — indicates the local-first mode and keeps the UI
  messaging accurate when cloud features are unavailable.
- **Desktop notifications** — stores the preference for future native
  notification delivery. In 0.4.0, cloud notifications are shown in the in-app
  popover; no scheduled background notification is delivered while the app is
  closed.
- **Reduce motion** — same as the appearance toggle.
- **Suggest priority automatically** — applies local rules at import time and
  sets `priority_source = "auto"`. Turn it off to start new mail at the
  **Default priority** instead. Manual importance and priority choices always
  win and are never overwritten.
- **Default priority for new mail** — `Low`, `Normal`, `High`, or `Urgent`.
- **Refresh suggestions** — recomputes automatic priorities for existing mail
  without touching messages you changed yourself.

### Senders and domains

**Settings → Senders & domains** manages the local blocklist and trusted
allowlist.

- **Block** rules filter matching mail automatically. A block can target one
  address (`name@example.com`) or a whole domain (`example.com`, including
  subdomains).
- **Allow** rules always win. Use them to keep a trusted address or domain out
  of the blocklist, even when a broader domain rule would match.
- **Blocked mail goes to** — choose **Spam** or **Trash**. Changing this
  re-applies the rules to already-blocked messages.
- **Apply rules** — re-evaluates every local message against the current rules.
  Removing a rule returns messages it had auto-filtered to **Inbox** unless you
  moved them yourself.
- Blocked messages show a **Blocked** chip in the list and the reader, with the
  matching address or domain.

Rules are stored in the local SQLite database and apply to incoming IMAP mail at
import time. Allow rules are evaluated before block rules.

### Sender reputation

The reader shows a reputation card under the message header. It classifies the
sender domain as **Trusted**, **Neutral**, **Suspicious**, or **Malicious** and
lists the reasons.

- **Trusted** — the sender is on your allowlist or uses an official domain for a
  known brand.
- **Neutral** — there is no negative signal. New and unknown domains stay here;
  they are never treated as malicious without evidence.
- **Suspicious** and **Malicious** — raised by block rules, brand impersonation
  or punycode, failed SPF/DKIM/DMARC checks, or threat-intelligence verdicts
  cached by Shield.
- **SPF / DKIM / DMARC** badges show the authentication results from the message
  headers when the provider supplies them. Missing results are shown as `n/a`.
- **Refresh** recomputes the reputation for the open message, including any new
  Shield results.
- **Report incorrect assessment** neutralizes the classification and stores your
  note. It stays Neutral until new intelligence produces a higher score.

**Security → Refresh sender reputations** recomputes the cached reputation for
every sender domain. Reputations are stored in SQLite and survive restarts.

### Attachment security

Elyra inspects attachments from their real bytes instead of trusting the file
name or the MIME type the sender declared. Local analysis always stays on this
device.

- **Risk badges** — **Scanned**, **Suspicious**, or **Malicious**. The reader
  lists the reasons, for example an executable or script, a disk image, a
  macro-enabled Office file, an archive, HTML or SVG content, a double extension
  such as `invoice.pdf.exe`, bidirectional-override characters that hide the real
  extension, or a detected type that contradicts the declared one.
- **Detected type** — the real content type when it can be sniffed, shown
  alongside the declared type when they differ.
- **Scan** — re-runs local analysis, plus ClamAV when it is enabled.
- **VirusTotal** — uploads the file only after you confirm. Uploads require a
  stored VirusTotal key, the **Allow VirusTotal file analysis** setting, and
  explicit per-file consent. Clean results never downgrade a local warning.
- **Download** — asks for confirmation when the file is suspicious or malicious.
  Elyra never opens, executes, or auto-runs attachments.

**Settings → Attachment security** controls the behavior:

- **Scan attachments automatically** — classify new mail at import.
- **Use local ClamAV** — run `clamscan` on stored attachment content. Set the
  **ClamAV command** to the executable name or a full path.
- **Allow VirusTotal file analysis** — enables the per-file upload button.
- **Maximum scan size (MB)** — larger files are still classified locally but are
  not sent to ClamAV or VirusTotal.
- **Rescan attachments** — re-runs the local scan over every stored attachment.

Attachments without stored content (metadata only) are still classified from
their name and declared type, and the card says so. Scans and settings persist in
SQLite across restarts.

### Mail accounts

See [Mail accounts](#mail-accounts).

### Data

- **Reset demo mailbox** — restores the seeded messages. This replaces the
  current demo content; real IMAP-synced messages should be backed up first.
- **Export mailbox** — downloads a JSON export of the local mailbox through the
  Rust `export_mailbox` command. The export includes folders, messages, and
  metadata. Treat it as sensitive: it contains message bodies.
- **Refresh diagnostics** — reloads the diagnostics panel.

### Sync and offline

The **Sync & offline** section keeps local mail readable when the network is
unavailable:

- **Work offline** stops the outbox from sending and prevents new IMAP fetches.
- **Queue a test message** stores a message in the local outbox without sending
  it. Useful for confirming that the queue works.
- **Send queued mail** flushes the outbox. Successful messages are removed;
  failures increment an attempt counter and are kept for retry.
- **Refresh status** reloads queued/failed/blocked counts and the last error.
- Each outbox row offers **Retry** and **Discard**. A message is marked blocked
  after five failed attempts; retry resets the counter.

The browser preview can queue messages but cannot flush them through SMTP; the
desktop build with a configured account or server relay is required to send.

### Diagnostics

The panel reports:

- Application version and engine name.
- Platform and architecture.
- SQLite version.
- Database path and size.
- Cloud API configuration state.
- OS keychain availability.
- Mailbox counts.

If the Rust command is unavailable, the panel shows an error instead of inventing
a healthy result.

The **Diagnostics & privacy** section adds:

- **Allow diagnostic logging** — controls whether new events are stored locally.
- **Enable optional crash reporting** — records a preference for a future
  reporting endpoint. Elyra 0.4.0 has no upload endpoint and sends nothing
  automatically.
- **Export diagnostic report** — downloads a JSON report containing app and
  database diagnostics plus redacted events. Secrets, email addresses, URL
  query strings and long opaque tokens are removed before an event is saved.
- **Refresh events** and **Clear log** — inspect or remove local diagnostic
  history. Logs are capped at 500 events and 30 days.

Unexpected JavaScript errors are written to the same local log, and the UI shows
a single non-blocking warning so you know where to look.

## Shield and integrations

Open the **Security** view. The page combines Elyra Shield, threat
intelligence, identity keys, and the local encryption lab.

### External link scanning

**External link scanning** is on by default. Turn it off to stop every external
provider call. Local Shield checks still run.

### Threat intelligence keys

- **VirusTotal** — create a free API key in your VirusTotal profile and paste it
  into the VirusTotal card. Elyra validates the key before enabling it.
- **Google Safe Browsing** — create a Google Cloud API key with the Safe
  Browsing API enabled and paste it into the Google Safe Browsing card.
- **Replace key**, **Test**, **Disable**, and **Remove key** manage the stored
  key. Keys live in the Windows Credential Manager and are never shown again.
- If a provider rejects the key or hits a quota, the card shows the error and
  the message still opens. Provider failures do not block mail.

### Reading Shield results

- A shield icon on a message row means Shield flagged it as Suspicious or
  Malicious.
- Opening a message shows a banner with the risk, score, reasons, and link
  verdicts.
- Malicious links are blocked. Suspicious or unscanned links ask for
  confirmation before opening.
- **Report false positive** records the report locally and updates the banner.
- The summary shows analyzed, malicious, suspicious, false-positive, and
  connected-provider counts.

### Identity keys

- **Generate identity key** creates an Ed25519 key pair. The private key is
  stored in the Windows Credential Manager; only the public key and fingerprint
  are stored in SQLite.
- **Copy public key** copies the armored public key.
- **Rotate** creates a new active key and retires the old one.
- **Revoke** marks the key revoked and deletes its private material.
- **Import public key** stores a contact's public key for future signing and
  encryption work.

### Encrypted envelopes (native desktop preview)

The native desktop build also contains versioned message-envelope primitives:

- A random per-message key is sealed to each verified recipient X25519 key with
  RFC 9180 HPKE (`DHKEM(X25519, HKDF-SHA256)`, `HKDF-SHA256`, `AES-256-GCM`).
- The complete envelope is signed with Ed25519, and the payload is authenticated
  with the envelope metadata as additional data.
- Private signing and encryption keys stay in the Windows Credential Manager;
  SQLite stores only public keys, fingerprints, and trust state.
- Recipients must be verified before encryption. Changed, revoked, and unverified
  keys block sending; opening an envelope requires a verified sender key.
- Unsupported versions, invalid signatures, wrong recipient keys, and tampered
  ciphertext fail without showing plaintext.

This is a protocol preview, not automatic E2EE for mailbox mail in 0.4.0. The
compose/send path, server relay, and attachments do not yet use envelopes, so do
not assume a message is end-to-end encrypted. The full design, guarantees, and
limitations are in [E2EE architecture](security/e2ee.md).

## Security Lab

The **Security** mailbox is a Space. The crypto panel runs real Rust commands:

- **Generate key** — creates a 32-byte key with OS randomness. The demo key is
  held in memory only.
- **Encrypt** — AES-256-GCM with a fresh 96-bit nonce, returned as a base64
  payload containing the nonce and ciphertext.
- **Decrypt** — validates the authentication tag and rejects malformed,
  tampered, or wrong-key payloads.
- **Fingerprint** — SHA-256 fingerprint for display and comparison.

The lab is a demonstration of the crypto boundary, not a full key-management
system. Production mailbox encryption at rest is not enabled in 0.4.0. See
[Security](security.md) for the exact guarantees and gaps.

## Changes

The **Changes** tab reads the Rust changelog command and renders shipped and
planned entries. It is a convenience view; [`changes.md`](../changes.md) and
[`task.md`](../task.md) remain the authoritative project logs.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+K` / `Cmd+K` | Open the command palette |
| `Arrow Up` / `Arrow Down` | Move through command palette results |
| `Enter` | Run the selected command |
| `Escape` | Close compose, command palette, notification popover, auth modal, or reader |
| `Tab` / `Shift+Tab` | Move focus through controls |

The command palette includes compose, refresh, quick filters (including
important, high-priority, and urgent), priority-suggestion refresh, mark mailbox
read, export, notifications, and settings.

## Local-only mode

Elyra works without a server or a mail account:

- The demo mailbox is seeded locally.
- Search, filters, drafts, stars, moves, themes, and the Security Lab work.
- Sending requires either a mail account, environment SMTP, or a signed-in
  cloud session with a relay-enabled server.
- IMAP sync requires a configured account and network access.
- Notifications require Elyra Cloud.

## Data and privacy summary

- Mailbox data is local. Elyra does not upload message bodies to its server.
- Cloud sessions and mail passwords are in the OS keychain.
- The server stores account metadata, sync cursors, notifications, and optional
  relay configuration.
- IMAP and SMTP require TLS or STARTTLS.
- HTML email is not rendered as raw HTML in the reader; text is escaped.
- Audit records contain event type, outcome, and details only, capped at 500
  records. Keys, tokens, bodies, and passwords are never written to the audit
  log.

For the complete model, read [Security](security.md).
