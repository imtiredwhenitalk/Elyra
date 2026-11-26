//! Local mailbox engine.
//!
//! Every mailbox decision lives here: schema migrations, demo seeding,
//! filtering, search, stats, drafts, bulk actions and export. The frontend is a
//! thin renderer that calls these Tauri commands.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDateTime, Utc};
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use serde::Serialize;
use tauri::AppHandle;

use crate::storage;

const FOLDERS: &[(&str, &str)] = &[
    ("Inbox", "system"),
    ("Starred", "system"),
    ("Snoozed", "system"),
    ("Sent", "system"),
    ("Drafts", "system"),
    ("Archive", "system"),
    ("Spam", "system"),
    ("Trash", "system"),
    ("Important", "label"),
    ("Updates", "label"),
    ("Security", "label"),
    ("Personal", "label"),
];

const COLORS: &[&str] = &["coral", "gold", "teal", "blue", "violet", "moss"];

#[derive(Debug, Clone, Serialize)]
pub struct MessageSummary {
    pub id: i64,
    pub sender: String,
    pub initials: String,
    pub subject: String,
    pub preview: String,
    pub time: String,
    pub received_at: String,
    pub folder: String,
    pub tag: String,
    pub color: String,
    pub is_read: bool,
    pub is_starred: bool,
    pub is_important: bool,
    pub priority: String,
    pub priority_source: String,
    pub priority_reason: Option<String>,
    pub blocked_by: Option<String>,
    pub has_attachments: bool,
    pub snoozed_until: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentInfo {
    pub id: i64,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub detected_mime: String,
    pub kind: String,
    pub risk: String,
    pub score: i64,
    pub reasons: Vec<String>,
    pub sha256: Option<String>,
    pub content_available: bool,
    pub scan_source: String,
    pub scanned_at: Option<String>,
    pub consent_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageDetail {
    pub message: MessageSummary,
    pub recipient: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MailboxPage {
    pub messages: Vec<MessageSummary>,
    pub total: i64,
    pub unread: i64,
    pub starred: i64,
    pub has_more: bool,
    pub filter: String,
    pub quick_filter: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MailboxStats {
    pub total: i64,
    pub unread: i64,
    pub starred: i64,
    pub important: i64,
    pub high_priority: i64,
    pub attachments: i64,
    pub trash: i64,
    pub spam: i64,
    pub drafts: i64,
    pub snoozed: i64,
    pub accounts: i64,
    pub storage_bytes: i64,
    pub last_sync_at: Option<String>,
    pub folder_counts: BTreeMap<String, i64>,
    pub folder_unread: BTreeMap<String, i64>,
}

fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn time_ago(minutes: i64) -> String {
    (Utc::now() - Duration::minutes(minutes))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

fn initials_for(sender: &str) -> String {
    let mut initials = String::new();
    for part in sender.split(|character: char| !character.is_alphanumeric()) {
        if let Some(character) = part.chars().next() {
            initials.extend(character.to_uppercase());
            if initials.chars().count() >= 2 {
                break;
            }
        }
    }
    if initials.is_empty() {
        "?".to_string()
    } else {
        initials
    }
}

fn color_for(sender: &str) -> String {
    let seed = sender.bytes().map(|byte| byte as usize).sum::<usize>();
    COLORS[seed % COLORS.len()].to_string()
}

pub(crate) fn preview_of(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() > 150 {
        let truncated = compact.chars().take(147).collect::<String>();
        format!("{truncated}...")
    } else {
        compact
    }
}

fn display_time(received_at: &str) -> String {
    let parsed = NaiveDateTime::parse_from_str(received_at, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(received_at, "%Y-%m-%d %H:%M:%S%.f"));
    let Ok(naive) = parsed else {
        return received_at.to_string();
    };
    let utc = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc);
    let local = utc.with_timezone(&Local);
    let now = Local::now();
    let delta = now.signed_duration_since(local);

    if delta.num_minutes() < 1 {
        "Just now".to_string()
    } else if delta.num_minutes() < 60 {
        format!("{}m ago", delta.num_minutes())
    } else if local.date_naive() == now.date_naive() {
        local.format("%H:%M").to_string()
    } else if local.date_naive() == (now - Duration::days(1)).date_naive() {
        "Yesterday".to_string()
    } else if local.year() == now.year() {
        local.format("%b %-d").to_string()
    } else {
        local.format("%b %-d, %Y").to_string()
    }
}

fn has_column(database: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut statement = database
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| error.to_string())?;
    let mut rows = statement.query([]).map_err(|error| error.to_string())?;
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        let name: String = row.get(1).map_err(|error| error.to_string())?;
        if name.eq_ignore_ascii_case(column) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn ensure_column(
    database: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    if !has_column(database, table, column)? {
        database
            .execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
                [],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn ensure_folders(database: &Connection) -> Result<(), String> {
    for (name, kind) in FOLDERS {
        database
            .execute(
                "INSERT OR IGNORE INTO folders (name, kind) VALUES (?1, ?2)",
                params![name, kind],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn folder_id(database: &Connection, name: &str) -> Result<i64, String> {
    database
        .query_row(
            "SELECT id FROM folders WHERE lower(name) = lower(?1)",
            params![name],
            |row| row.get(0),
        )
        .map_err(|_| format!("Unknown folder: {name}"))
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    storage::initialize(app)?;
    let database = storage::connection(app)?;
    let had_priority = has_column(&database, "emails", "priority")?;

    for (column, definition) in [
        ("preview", "TEXT NOT NULL DEFAULT ''"),
        ("tag", "TEXT NOT NULL DEFAULT 'Personal'"),
        ("recipient", "TEXT NOT NULL DEFAULT ''"),
        ("sender_address", "TEXT NOT NULL DEFAULT ''"),
        ("has_attachments", "INTEGER NOT NULL DEFAULT 0"),
        ("snoozed_until", "TEXT"),
        ("thread_id", "TEXT"),
        ("size_bytes", "INTEGER NOT NULL DEFAULT 0"),
        ("updated_at", "TEXT NOT NULL DEFAULT ''"),
        ("deleted_at", "TEXT"),
        ("is_important", "INTEGER NOT NULL DEFAULT 0"),
        ("priority", "TEXT NOT NULL DEFAULT 'normal'"),
        ("priority_source", "TEXT NOT NULL DEFAULT 'auto'"),
        ("priority_reason", "TEXT"),
        ("blocked_by", "TEXT"),
        ("auth_results", "TEXT NOT NULL DEFAULT ''"),
    ] {
        ensure_column(&database, "emails", column, definition)?;
    }

    if !had_priority {
        database
            .execute(
                "UPDATE emails SET is_important = 1
                 WHERE is_important = 0
                   AND (lower(tag) = 'important'
                        OR folder_id IN (SELECT id FROM folders WHERE lower(name) = 'important'))",
                [],
            )
            .map_err(|error| error.to_string())?;
    }

    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS attachments (
                id INTEGER PRIMARY KEY,
                email_id INTEGER NOT NULL REFERENCES emails(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
                size_bytes INTEGER NOT NULL DEFAULT 0,
                stored_path TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_emails_folder ON emails(folder_id);
             CREATE INDEX IF NOT EXISTS idx_emails_received ON emails(received_at DESC);
             CREATE INDEX IF NOT EXISTS idx_emails_starred ON emails(is_starred);
             CREATE INDEX IF NOT EXISTS idx_attachments_email ON attachments(email_id);",
        )
        .map_err(|error| error.to_string())?;

    // Make sure attachment scanning columns exist before demo seeding stores
    // attachment content. `attachments::initialize` re-checks them later.
    crate::attachments::ensure_columns(&database)?;

    ensure_folders(&database)?;

    for (key, value) in [
        ("theme", "light"),
        ("accent", "moss"),
        ("density", "comfortable"),
        ("display_name", "Alex Smith"),
        ("domain_handle", "alex@elyra.com"),
        ("signature", "Sent from Elyra"),
        ("local_only", "true"),
        ("desktop_notifications", "true"),
        ("reduce_motion", "false"),
        ("auto_priority", "true"),
        ("priority_default", "normal"),
        ("blocked_sender_action", "spam"),
    ] {
        database
            .execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|error| error.to_string())?;
    }

    database
        .execute(
            "INSERT OR IGNORE INTO accounts (id, provider, email, display_name)
             VALUES (1, 'elyra', 'alex@elyra.com', 'Alex Smith')",
            [],
        )
        .map_err(|error| error.to_string())?;

    seed_if_empty(&database)?;
    database
        .execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('last_sync_at', ?1)",
            params![now_string()],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn account_id(database: &Connection) -> i64 {
    database
        .query_row(
            "SELECT id FROM accounts WHERE lower(email) = 'alex@elyra.com'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(1)
}

fn demo_address(name: &str) -> String {
    let slug = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '.'
            }
        })
        .collect::<String>();
    let slug = slug.trim_matches('.');
    format!("{slug}@example.com")
}

#[allow(clippy::too_many_arguments)]
fn insert_message(
    database: &Connection,
    provider_id: &str,
    sender: &str,
    recipient: &str,
    subject: &str,
    body: &str,
    folder: &str,
    tag: &str,
    is_read: bool,
    is_starred: bool,
    received_at: &str,
    snoozed_until: Option<&str>,
) -> Result<i64, String> {
    let folder = folder_id(database, folder)?;
    let preview = preview_of(body);
    database
        .execute(
            "INSERT INTO emails (
                account_id, provider_id, sender, subject, body_text, preview, folder_id,
                is_read, is_starred, received_at, tag, recipient, snoozed_until,
                updated_at, size_bytes
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?10, ?14)",
            params![
                account_id(database),
                provider_id,
                sender,
                subject,
                body,
                preview,
                folder,
                if is_read { 1 } else { 0 },
                if is_starred { 1 } else { 0 },
                received_at,
                tag,
                recipient,
                snoozed_until,
                body.len() as i64,
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(database.last_insert_rowid())
}

pub(crate) fn insert_attachment(
    database: &Connection,
    email_id: i64,
    name: &str,
    mime_type: &str,
    size_bytes: i64,
) -> Result<(), String> {
    insert_attachment_with_content(database, email_id, name, mime_type, size_bytes, None)
}

pub(crate) fn insert_attachment_with_content(
    database: &Connection,
    email_id: i64,
    name: &str,
    mime_type: &str,
    size_bytes: i64,
    content: Option<&[u8]>,
) -> Result<(), String> {
    let content_available = content.map(|bytes| !bytes.is_empty()).unwrap_or(false);
    database
        .execute(
            "INSERT INTO attachments
                (email_id, name, mime_type, size_bytes, content, content_available)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                email_id,
                name,
                mime_type,
                size_bytes,
                content,
                if content_available { 1 } else { 0 }
            ],
        )
        .map_err(|error| error.to_string())?;
    database
        .execute(
            "UPDATE emails SET has_attachments = 1 WHERE id = ?1",
            params![email_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn seed_if_empty(database: &Connection) -> Result<(), String> {
    let count: i64 = database
        .query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if count > 0 {
        return Ok(());
    }

    let demo = [
        (
            "Maya Chen",
            "alex@elyra.com",
            "The quieter way to work",
            "Hi Alex,\n\nI've been thinking about the new rhythm we discussed. Fewer meetings, longer focus blocks, and a calmer inbox would give the team room to do their best work.\n\nI put together a short proposal with three experiments we could run next month. Nothing heavy, just enough to learn what actually helps.\n\nLet me know what you think.\n\nMaya",
            "Important",
            "Important",
            false,
            true,
            12,
            true,
        ),
        (
            "Elyra Security",
            "alex@elyra.com",
            "New sign-in protected",
            "A new sign-in was protected with your device key.\n\nDevice: Windows 11 desktop\nLocation: this device\nTime: a few minutes ago\n\nIf this was you, no action is needed. If it wasn't, review your active sessions from Settings and rotate your recovery key.",
            "Security",
            "Security",
            false,
            false,
            48,
            false,
        ),
        (
            "Linear",
            "alex@elyra.com",
            "Your weekly product digest",
            "Here is what changed across your active projects this week:\n\n- Inbox workspace reached the review milestone\n- 14 issues were completed\n- The privacy settings pass is ready for testing\n\nOpen Linear to see the full breakdown.",
            "Updates",
            "Updates",
            false,
            false,
            220,
            false,
        ),
        (
            "Jon Bell",
            "alex@elyra.com",
            "Re: Q4 planning notes",
            "The smaller scope feels right. I added two notes to the doc: keep the first release focused on the desktop inbox, and move provider sync behind a feature flag.\n\nIf we protect the core experience first, the rest gets much easier to explain.",
            "Important",
            "Important",
            false,
            true,
            330,
            true,
        ),
        (
            "Noah Williams",
            "alex@elyra.com",
            "Coffee next week?",
            "Tuesday or Thursday both work on my end. There is a new place near the studio with a quiet back room and very good cardamom buns.\n\nNo agenda. Just a proper catch-up.",
            "Inbox",
            "Personal",
            true,
            true,
            1_440,
            false,
        ),
        (
            "Daily Offers",
            "alex@elyra.com",
            "You have been selected",
            "Claim your exclusive reward before midnight. This message was automatically filtered into Spam by the Elyra privacy engine.",
            "Spam",
            "Spam",
            true,
            false,
            1_600,
            false,
        ),
        (
            "Maya Chen",
            "alex@elyra.com",
            "Old project notes",
            "Moved to trash during inbox cleanup. The archived copy is still available from the project workspace.",
            "Trash",
            "Trash",
            true,
            false,
            8_300,
            false,
        ),
        (
            "Elyra Team",
            "alex@elyra.com",
            "Welcome to Elyra",
            "Thanks for joining the Elyra community. Your inbox is now a local-first workspace: search, folders, drafts and encryption all run on this device.\n\nStart with the command palette (Ctrl+K), connect an account, or send yourself a first note.",
            "Inbox",
            "Welcome",
            true,
            false,
            9_100,
            false,
        ),
        (
            "Elyra Security",
            "alex@elyra.com",
            "Password changed successfully",
            "Your password was changed successfully. If you did not make this change, contact support immediately and revoke active sessions.",
            "Security",
            "Security",
            true,
            false,
            12_200,
            false,
        ),
        (
            "Linear",
            "alex@elyra.com",
            "New project created",
            "A new project has been created in your workspace. The project inherits your team defaults for cycles, labels and notifications.",
            "Updates",
            "Updates",
            true,
            false,
            14_000,
            false,
        ),
        (
            "Jon Bell",
            "alex@elyra.com",
            "Meeting rescheduled",
            "The meeting has been moved to next week. The calendar invite is updated and the notes doc is unchanged.",
            "Important",
            "Important",
            true,
            false,
            16_000,
            false,
        ),
        (
            "Proton",
            "alex@elyra.com",
            "Your weekly privacy report",
            "This week Elyra blocked 18 trackers, kept 42 messages local, and verified every encryption round-trip.\n\nYour privacy score is steady at 100%.",
            "Security",
            "Security",
            false,
            false,
            95,
            false,
        ),
        (
            "GitHub",
            "alex@elyra.com",
            "Release 0.2.0 is ready",
            "The new desktop build includes the Rust mailbox engine, a refreshed interface, a proper Windows installer and an improved public website.\n\nReview the release notes before publishing.",
            "Updates",
            "Updates",
            true,
            false,
            720,
            false,
        ),
        (
            "Sofia Marin",
            "alex@elyra.com",
            "Design review notes",
            "I left a few comments on the new message list. The hierarchy is much calmer now. My only ask is to keep the reading pane generous on wide screens and collapse it gracefully on laptops.",
            "Inbox",
            "Personal",
            false,
            false,
            180,
            false,
        ),
        (
            "Elyra Team",
            "alex@elyra.com",
            "Snoozed until tomorrow",
            "This message was snoozed for one day so the inbox stays focused. It will return automatically.",
            "Snoozed",
            "Snoozed",
            true,
            false,
            2_000,
            false,
        ),
        (
            "Alex Smith",
            "maya@example.com",
            "Re: The quieter way to work",
            "Maya,\n\nI love the direction. Let's run the focus-block experiment first and keep the rest lightweight.\n\nAlex",
            "Sent",
            "Sent",
            true,
            false,
            25,
            false,
        ),
        (
            "Alex Smith",
            "team@elyra.com",
            "Launch notes",
            "Draft outline:\n\n1. Rust mailbox core\n2. New icon system\n3. MSI release pipeline\n4. Website refresh\n\nStill needs a final pass on the screenshots.",
            "Drafts",
            "Draft",
            true,
            false,
            40,
            false,
        ),
        (
            "Tomas Adler",
            "alex@elyra.com",
            "Archived: office move checklist",
            "Everything from the old office is packed. The archive copy is here for reference.",
            "Archive",
            "Archive",
            true,
            false,
            20_000,
            false,
        ),
    ];

    for (index, demo) in demo.iter().enumerate() {
        let received_at = time_ago(demo.8);
        let snoozed_until = if demo.4.eq_ignore_ascii_case("Snoozed") {
            Some(time_ago(-1_440))
        } else {
            None
        };
        let snoozed_until = snoozed_until.as_deref();
        let email_id = insert_message(
            database,
            &format!("seed-{index}"),
            demo.0,
            demo.1,
            demo.2,
            demo.3,
            demo.4,
            demo.5,
            demo.6,
            demo.7,
            &received_at,
            snoozed_until,
        )?;
        if demo.9 {
            if index % 2 == 0 {
                insert_attachment(database, email_id, "message-notes.pdf", "application/pdf", 248_320)?;
            } else {
                insert_attachment(database, email_id, "planning-brief.pdf", "application/pdf", 512_000)?;
                insert_attachment(database, email_id, "brand-board.png", "image/png", 1_842_000)?;
            }
        }
        if demo.4.eq_ignore_ascii_case("Spam") {
            insert_attachment(
                database,
                email_id,
                "claim-form.pdf.exe",
                "application/pdf",
                1_204_224,
            )?;
        }
        let _ = crate::priority::apply_auto_priority(database, email_id, true);
        let auth_results = if demo.4.eq_ignore_ascii_case("Spam") {
            r#"{"spf":"fail","dkim":"fail","dmarc":"fail","source":"demo"}"#
        } else {
            r#"{"spf":"pass","dkim":"pass","dmarc":"pass","source":"demo"}"#
        };
        let _ = database.execute(
            "UPDATE emails SET sender_address = ?2, auth_results = ?3 WHERE id = ?1",
            params![email_id, demo_address(demo.0), auth_results],
        );
        let _ = crate::senders::apply_to_email(database, email_id);
        // Classify seeded attachments now so a demo reset shows the same risk
        // cards as a freshly imported mailbox.
        let _ = crate::attachments::scan_email_attachments(database, email_id);
    }

    Ok(())
}

pub(crate) fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageSummary> {
    let sender: String = row.get(1)?;
    let received_at: String = row.get(8)?;
    Ok(MessageSummary {
        id: row.get(0)?,
        initials: initials_for(&sender),
        color: color_for(&sender),
        sender,
        subject: row.get(2)?,
        preview: row.get(3)?,
        folder: row.get(4)?,
        tag: row.get(5)?,
        is_read: row.get::<_, i64>(6)? != 0,
        is_starred: row.get::<_, i64>(7)? != 0,
        is_important: row.get::<_, i64>(11)? != 0,
        priority: row.get(12)?,
        priority_source: row.get(13)?,
        priority_reason: row.get(14)?,
        blocked_by: row.get(15)?,
        has_attachments: row.get::<_, i64>(9)? != 0,
        snoozed_until: row.get(10)?,
        time: display_time(&received_at),
        received_at,
    })
}

fn build_filters(filter: &str, quick_filter: &str, query: &str) -> (String, Vec<Value>) {
    let mut conditions = vec!["e.deleted_at IS NULL".to_string()];
    let mut values = Vec::new();

    match filter.to_ascii_lowercase().as_str() {
        "inbox" => conditions.push(
            "lower(f.name) NOT IN ('spam', 'trash', 'sent', 'drafts', 'snoozed', 'archive') \
             AND (e.snoozed_until IS NULL OR e.snoozed_until <= datetime('now'))"
                .to_string(),
        ),
        "starred" => conditions
            .push("e.is_starred = 1 AND lower(f.name) NOT IN ('trash', 'spam')".to_string()),
        "important" => conditions.push(
            "e.is_important = 1 AND lower(f.name) NOT IN ('trash', 'spam')".to_string(),
        ),
        "all" | "" => {}
        "snoozed" => conditions.push("lower(f.name) = 'snoozed'".to_string()),
        other => {
            conditions.push("lower(f.name) = ?".to_string());
            values.push(Value::Text(other.to_string()));
        }
    }

    match quick_filter.to_ascii_lowercase().as_str() {
        "unread" => conditions.push("e.is_read = 0".to_string()),
        "starred" => conditions.push("e.is_starred = 1".to_string()),
        "important" => conditions.push("e.is_important = 1".to_string()),
        "high" => conditions.push("e.priority IN ('high', 'urgent')".to_string()),
        "urgent" => conditions.push("e.priority = 'urgent'".to_string()),
        "attachments" => conditions.push("e.has_attachments = 1".to_string()),
        _ => {}
    }

    let query = query.trim();
    if !query.is_empty() {
        conditions.push(
            "(e.sender LIKE ? OR e.subject LIKE ? OR e.preview LIKE ? OR e.body_text LIKE ?)"
                .to_string(),
        );
        let pattern = format!("%{query}%");
        for _ in 0..4 {
            values.push(Value::Text(pattern.clone()));
        }
    }

    (conditions.join(" AND "), values)
}

pub(crate) fn summary_sql() -> &'static str {
    "SELECT e.id, e.sender, e.subject, e.preview, f.name, e.tag, e.is_read,
            e.is_starred, e.received_at, e.has_attachments, e.snoozed_until,
            e.is_important, e.priority, e.priority_source, e.priority_reason,
            e.blocked_by
     FROM emails e JOIN folders f ON f.id = e.folder_id"
}

pub(crate) fn stats(app: &AppHandle) -> Result<MailboxStats, String> {
    let database = storage::connection(app)?;
    let total: i64 = database
        .query_row(
            "SELECT COUNT(*) FROM emails WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let unread: i64 = database
        .query_row(
            "SELECT COUNT(*) FROM emails WHERE deleted_at IS NULL AND is_read = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let starred: i64 = database
        .query_row(
            "SELECT COUNT(*) FROM emails WHERE deleted_at IS NULL AND is_starred = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let important: i64 = database
        .query_row(
            "SELECT COUNT(*) FROM emails e JOIN folders f ON f.id = e.folder_id
             WHERE e.deleted_at IS NULL AND e.is_important = 1
               AND lower(f.name) NOT IN ('trash', 'spam')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let high_priority: i64 = database
        .query_row(
            "SELECT COUNT(*) FROM emails e JOIN folders f ON f.id = e.folder_id
             WHERE e.deleted_at IS NULL AND e.priority IN ('high', 'urgent')
               AND lower(f.name) NOT IN ('trash', 'spam')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let attachments: i64 = database
        .query_row("SELECT COUNT(*) FROM attachments", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let accounts: i64 = database
        .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let last_sync_at: Option<String> = database
        .query_row(
            "SELECT value FROM settings WHERE key = 'last_sync_at'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let database_bytes = storage::database_path(app)
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0);

    let mut folder_counts = BTreeMap::new();
    let mut folder_unread = BTreeMap::new();
    let mut statement = database
        .prepare(
            "SELECT lower(f.name), COUNT(e.id),
                    COALESCE(SUM(CASE WHEN e.is_read = 0 THEN 1 ELSE 0 END), 0)
             FROM emails e JOIN folders f ON f.id = e.folder_id
             WHERE e.deleted_at IS NULL
             GROUP BY lower(f.name)",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (folder, count, unread_count) = row.map_err(|error| error.to_string())?;
        folder_counts.insert(folder.clone(), count);
        folder_unread.insert(folder, unread_count);
    }

    let trash = folder_counts.get("trash").copied().unwrap_or(0);
    let spam = folder_counts.get("spam").copied().unwrap_or(0);
    let drafts = folder_counts.get("drafts").copied().unwrap_or(0);
    let snoozed = folder_counts.get("snoozed").copied().unwrap_or(0);
    Ok(MailboxStats {
        total,
        unread,
        starred,
        important,
        high_priority,
        attachments,
        trash,
        spam,
        drafts,
        snoozed,
        accounts,
        storage_bytes: database_bytes,
        last_sync_at,
        folder_counts,
        folder_unread,
    })
}

fn message_detail(database: &Connection, id: i64) -> Result<MessageDetail, String> {
    let mut statement = database
        .prepare(
            "SELECT e.id, e.sender, e.subject, e.preview, f.name, e.tag, e.is_read,
                    e.is_starred, e.received_at, e.has_attachments, e.snoozed_until,
                    e.is_important, e.priority, e.priority_source, e.priority_reason,
                    e.blocked_by, e.recipient, e.body_text, e.body_html
             FROM emails e JOIN folders f ON f.id = e.folder_id
             WHERE e.id = ?1 AND e.deleted_at IS NULL",
        )
        .map_err(|error| error.to_string())?;
    let detail = statement
        .query_row(params![id], |row| {
            let summary = row_to_summary(row)?;
            Ok(MessageDetail {
                message: summary,
                recipient: row.get(16)?,
                body_text: row.get(17)?,
                body_html: row.get(18)?,
                attachments: Vec::new(),
            })
        })
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Message not found".to_string())?;

    let mut attachment_statement = database
        .prepare(
            "SELECT id, name, mime_type, size_bytes, detected_mime, kind, risk,
                    score, reasons, sha256, content_available, scan_source,
                    scanned_at, consent_required
             FROM attachments WHERE email_id = ?1 ORDER BY id",
        )
        .map_err(|error| error.to_string())?;
    let attachments = attachment_statement
        .query_map(params![id], |row| {
            Ok(AttachmentInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                mime_type: row.get(2)?,
                size_bytes: row.get(3)?,
                detected_mime: row.get(4)?,
                kind: row.get(5)?,
                risk: row.get(6)?,
                score: row.get(7)?,
                reasons: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                sha256: row.get(9)?,
                content_available: row.get::<_, i64>(10)? != 0,
                scan_source: row.get(11)?,
                scanned_at: row.get(12)?,
                consent_required: row.get::<_, i64>(13)? != 0,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    Ok(MessageDetail {
        attachments,
        ..detail
    })
}

#[tauri::command]
pub fn mailbox_stats(app: AppHandle) -> Result<MailboxStats, String> {
    stats(&app)
}

#[tauri::command]
pub fn seed_demo_mailbox(app: AppHandle, force: Option<bool>) -> Result<MailboxStats, String> {
    let database = storage::connection(&app)?;
    if force.unwrap_or(false) {
        database
            .execute_batch("DELETE FROM attachments; DELETE FROM emails;")
            .map_err(|error| error.to_string())?;
    }
    seed_if_empty(&database)?;
    database
        .execute(
            "INSERT INTO settings (key, value) VALUES ('last_sync_at', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![now_string()],
        )
        .map_err(|error| error.to_string())?;
    stats(&app)
}

#[tauri::command]
pub fn reset_demo_mailbox(app: AppHandle) -> Result<MailboxStats, String> {
    seed_demo_mailbox(app, Some(true))
}

#[tauri::command]
pub fn refresh_mailbox(app: AppHandle) -> Result<MailboxStats, String> {
    let database = storage::connection(&app)?;
    database
        .execute(
            "INSERT INTO settings (key, value) VALUES ('last_sync_at', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![now_string()],
        )
        .map_err(|error| error.to_string())?;
    stats(&app)
}

#[tauri::command]
pub fn list_mailbox(
    app: AppHandle,
    filter: Option<String>,
    quick_filter: Option<String>,
    query: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<MailboxPage, String> {
    let database = storage::connection(&app)?;
    let filter = filter.unwrap_or_else(|| "inbox".to_string());
    let quick_filter = quick_filter.unwrap_or_else(|| "all".to_string());
    let query = query.unwrap_or_default();
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let offset = offset.unwrap_or(0).max(0);

    let (where_clause, mut values) = build_filters(&filter, &quick_filter, &query);
    values.push(Value::Integer(limit));
    values.push(Value::Integer(offset));
    let sql = format!(
        "{} WHERE {} ORDER BY e.received_at DESC, e.id DESC LIMIT ? OFFSET ?",
        summary_sql(),
        where_clause
    );
    let mut statement = database.prepare(&sql).map_err(|error| error.to_string())?;
    let messages = statement
        .query_map(params_from_iter(values), row_to_summary)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    let (count_where, count_values) = build_filters(&filter, &quick_filter, &query);
    let total: i64 = database
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM emails e JOIN folders f ON f.id = e.folder_id WHERE {count_where}"
            ),
            params_from_iter(count_values),
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;

    let snapshot = stats(&app)?;
    Ok(MailboxPage {
        has_more: offset + (messages.len() as i64) < total,
        messages,
        total,
        unread: snapshot.unread,
        starred: snapshot.starred,
        filter,
        quick_filter,
    })
}

#[tauri::command]
pub fn get_message(app: AppHandle, id: i64) -> Result<MessageDetail, String> {
    let database = storage::connection(&app)?;
    let detail = message_detail(&database, id)?;
    database
        .execute(
            "UPDATE emails SET is_read = 1, updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )
        .map_err(|error| error.to_string())?;
    Ok(detail)
}

#[tauri::command]
pub fn update_message(
    app: AppHandle,
    id: i64,
    is_read: Option<bool>,
    is_starred: Option<bool>,
    is_important: Option<bool>,
    priority: Option<String>,
    folder: Option<String>,
) -> Result<(), String> {
    let database = storage::connection(&app)?;
    let mut assignments = Vec::new();
    let mut values = Vec::new();

    if let Some(is_read) = is_read {
        assignments.push("is_read = ?");
        values.push(Value::Integer(if is_read { 1 } else { 0 }));
    }
    if let Some(is_starred) = is_starred {
        assignments.push("is_starred = ?");
        values.push(Value::Integer(if is_starred { 1 } else { 0 }));
    }
    if let Some(is_important) = is_important {
        assignments.push("is_important = ?");
        values.push(Value::Integer(if is_important { 1 } else { 0 }));
    }
    if let Some(priority) = priority {
        let priority = crate::priority::normalize_priority(&priority)
            .ok_or_else(|| "Unknown priority level".to_string())?;
        assignments.push("priority = ?");
        values.push(Value::Text(priority.to_string()));
        assignments.push("priority_source = 'manual'");
        assignments.push("priority_reason = 'Set manually.'");
    }
    if let Some(folder) = folder {
        assignments.push("folder_id = ?");
        values.push(Value::Integer(folder_id(&database, &folder)?));
        assignments.push("blocked_by = NULL");
    }
    if assignments.is_empty() {
        return Ok(());
    }
    assignments.push("updated_at = datetime('now')");
    values.push(Value::Integer(id));
    database
        .execute(
            &format!(
                "UPDATE emails SET {} WHERE id = ? AND deleted_at IS NULL",
                assignments.join(", ")
            ),
            params_from_iter(values),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_message(app: AppHandle, id: i64, permanent: Option<bool>) -> Result<(), String> {
    let database = storage::connection(&app)?;
    if permanent.unwrap_or(false) {
        database
            .execute("DELETE FROM emails WHERE id = ?1", params![id])
            .map_err(|error| error.to_string())?;
    } else {
        let trash = folder_id(&database, "Trash")?;
        database
            .execute(
                "UPDATE emails SET folder_id = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![trash, id],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn bulk_update_messages(
    app: AppHandle,
    ids: Vec<i64>,
    action: String,
    until: Option<String>,
) -> Result<i64, String> {
    if ids.is_empty() {
        return Ok(0);
    }
    if ids.len() > 500 {
        return Err("Select 500 messages or fewer at a time".to_string());
    }
    let database = storage::connection(&app)?;
    let placeholders = vec!["?"; ids.len()].join(", ");
    let mut values = Vec::new();
    let sql = match action.to_ascii_lowercase().as_str() {
        "read" => format!(
            "UPDATE emails SET is_read = 1, updated_at = datetime('now') WHERE id IN ({placeholders})"
        ),
        "unread" => format!(
            "UPDATE emails SET is_read = 0, updated_at = datetime('now') WHERE id IN ({placeholders})"
        ),
        "star" => format!(
            "UPDATE emails SET is_starred = 1, updated_at = datetime('now') WHERE id IN ({placeholders})"
        ),
        "unstar" => format!(
            "UPDATE emails SET is_starred = 0, updated_at = datetime('now') WHERE id IN ({placeholders})"
        ),
        "important" => format!(
            "UPDATE emails SET is_important = 1, updated_at = datetime('now') WHERE id IN ({placeholders})"
        ),
        "unimportant" => format!(
            "UPDATE emails SET is_important = 0, updated_at = datetime('now') WHERE id IN ({placeholders})"
        ),
        "priority-low" | "priority-normal" | "priority-high" | "priority-urgent" => {
            let priority = action.trim_start_matches("priority-");
            values.push(Value::Text(priority.to_ascii_lowercase()));
            format!(
                "UPDATE emails SET priority = ?, priority_source = 'manual',
                    priority_reason = 'Set manually.', updated_at = datetime('now')
                 WHERE id IN ({placeholders})"
            )
        }
        "delete" => format!("DELETE FROM emails WHERE id IN ({placeholders})"),
        "trash" | "archive" | "spam" | "inbox" | "restore" => {
            let folder = match action.to_ascii_lowercase().as_str() {
                "trash" => "Trash",
                "archive" => "Archive",
                "spam" => "Spam",
                _ => "Inbox",
            };
            values.push(Value::Integer(folder_id(&database, folder)?));
            format!(
                "UPDATE emails SET folder_id = ?, blocked_by = NULL, updated_at = datetime('now') WHERE id IN ({placeholders})"
            )
        }
        "snooze" => {
            values.push(Value::Integer(folder_id(&database, "Snoozed")?));
            values.push(Value::Text(
                until.unwrap_or_else(|| (Utc::now() + Duration::days(1)).format("%Y-%m-%d %H:%M:%S").to_string()),
            ));
            format!(
                "UPDATE emails SET folder_id = ?, snoozed_until = ?, blocked_by = NULL, updated_at = datetime('now') WHERE id IN ({placeholders})"
            )
        }
        other => return Err(format!("Unsupported action: {other}")),
    };
    values.extend(ids.iter().map(|id| Value::Integer(*id)));
    database
        .execute(&sql, params_from_iter(values))
        .map(|changed| changed as i64)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn mark_all_read(app: AppHandle, filter: Option<String>) -> Result<i64, String> {
    let database = storage::connection(&app)?;
    let filter = filter.unwrap_or_else(|| "inbox".to_string());
    let (where_clause, values) = build_filters(&filter, "all", "");
    database
        .execute(
            &format!(
                "UPDATE emails SET is_read = 1, updated_at = datetime('now')
                 WHERE id IN (SELECT e.id FROM emails e JOIN folders f ON f.id = e.folder_id
                 WHERE {where_clause})"
            ),
            params_from_iter(values),
        )
        .map(|changed| changed as i64)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn empty_trash(app: AppHandle) -> Result<i64, String> {
    let database = storage::connection(&app)?;
    let trash = folder_id(&database, "Trash")?;
    database
        .execute("DELETE FROM emails WHERE folder_id = ?1", params![trash])
        .map(|changed| changed as i64)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_draft(
    app: AppHandle,
    id: Option<i64>,
    recipient: String,
    subject: String,
    body: String,
) -> Result<i64, String> {
    let database = storage::connection(&app)?;
    let drafts = folder_id(&database, "Drafts")?;
    let now = now_string();
    if let Some(id) = id {
        let changed = database
            .execute(
                "UPDATE emails SET recipient = ?1, subject = ?2, body_text = ?3, preview = ?4,
                    updated_at = ?5 WHERE id = ?6 AND folder_id = ?7",
                params![recipient, subject, body, preview_of(&body), now, id, drafts],
            )
            .map_err(|error| error.to_string())?;
        if changed > 0 {
            return Ok(id);
        }
    }
    database
        .execute(
            "INSERT INTO emails (account_id, provider_id, sender, subject, body_text, preview,
                folder_id, is_read, is_starred, received_at, tag, recipient, updated_at, size_bytes)
             VALUES (NULL, ?1, 'Alex Smith', ?2, ?3, ?4, ?5, 1, 0, ?6, 'Draft', ?7, ?6, ?8)",
            params![
                format!("draft-{}", Utc::now().timestamp_nanos_opt().unwrap_or_default()),
                subject,
                body,
                preview_of(&body),
                drafts,
                now,
                recipient,
                body.len() as i64
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(database.last_insert_rowid())
}

#[tauri::command]
pub fn delete_draft(app: AppHandle, id: i64) -> Result<bool, String> {
    let database = storage::connection(&app)?;
    let drafts = folder_id(&database, "Drafts")?;
    database
        .execute(
            "DELETE FROM emails WHERE id = ?1 AND folder_id = ?2",
            params![id, drafts],
        )
        .map(|changed| changed > 0)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn search_suggestions(app: AppHandle, query: String, limit: Option<i64>) -> Result<Vec<String>, String> {
    let database = storage::connection(&app)?;
    let limit = limit.unwrap_or(6).clamp(1, 20);
    let pattern = format!("%{}%", query.trim());
    let mut suggestions = Vec::new();

    let mut contact_statement = database
        .prepare(
            "SELECT email FROM contacts WHERE email LIKE ?1 OR display_name LIKE ?1
             ORDER BY display_name LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let contacts = contact_statement
        .query_map(params![pattern, limit], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    for contact in contacts {
        suggestions.push(contact.map_err(|error| error.to_string())?);
    }

    let mut subject_statement = database
        .prepare(
            "SELECT DISTINCT subject FROM emails WHERE subject LIKE ?1 AND deleted_at IS NULL
             ORDER BY received_at DESC LIMIT ?2",
        )
        .map_err(|error| error.to_string())?;
    let subjects = subject_statement
        .query_map(params![pattern, limit], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?;
    for subject in subjects {
        suggestions.push(subject.map_err(|error| error.to_string())?);
    }
    suggestions.dedup();
    suggestions.truncate(limit as usize);
    Ok(suggestions)
}

#[tauri::command]
pub fn export_mailbox(app: AppHandle) -> Result<serde_json::Value, String> {
    let database = storage::connection(&app)?;
    let mut ids_statement = database
        .prepare("SELECT id FROM emails WHERE deleted_at IS NULL ORDER BY received_at DESC")
        .map_err(|error| error.to_string())?;
    let ids = ids_statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    let mut messages = Vec::new();
    for id in ids {
        messages.push(message_detail(&database, id)?);
    }

    let mut settings_statement = database
        .prepare("SELECT key, value FROM settings WHERE key NOT LIKE 'cloud_%'")
        .map_err(|error| error.to_string())?;
    let settings = settings_statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .map_err(|error| error.to_string())?;

    Ok(serde_json::json!({
        "format": "elyra-mailbox",
        "version": env!("CARGO_PKG_VERSION"),
        "exported_at": now_string(),
        "messages": messages,
        "settings": settings,
    }))
}

pub(crate) fn record_sent_email(
    app: &AppHandle,
    account: Option<i64>,
    sender: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> Result<(), String> {
    let database = storage::connection(app)?;
    let now = now_string();
    database
        .execute(
            "INSERT INTO emails (account_id, provider_id, sender, subject, body_text, preview,
                folder_id, is_read, is_starred, received_at, tag, recipient, updated_at, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 0, ?8, 'Sent', ?9, ?8, ?10)",
            params![
                account.unwrap_or_else(|| account_id(&database)),
                format!("sent-{}", Utc::now().timestamp_nanos_opt().unwrap_or_default()),
                sender,
                subject,
                body,
                preview_of(body),
                folder_id(&database, "Sent")?,
                now,
                to,
                body.len() as i64
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_are_stable() {
        assert_eq!(initials_for("Maya Chen"), "MC");
        assert_eq!(initials_for("Linear"), "L");
        assert_eq!(initials_for("Elyra Security"), "ES");
    }

    #[test]
    fn previews_are_trimmed() {
        let preview = preview_of(&"word ".repeat(80));
        assert!(preview.ends_with("..."));
        assert!(preview.chars().count() <= 150);
    }
}
