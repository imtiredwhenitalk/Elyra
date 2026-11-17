//! Sender and domain blocklists plus a trusted allowlist.
//!
//! Rules live in SQLite. Allow rules always win over block rules, so a trusted
//! address or domain is never filtered. Blocked mail is moved to Spam or Trash
//! according to the `blocked_sender_action` setting, and the matching rule is
//! recorded on the message so the interface can explain why it was filtered.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::AppHandle;

use crate::{mailbox, storage};

#[derive(Debug, Clone, Serialize)]
pub struct SenderRule {
    pub id: i64,
    pub kind: String,
    pub scope: String,
    pub value: String,
    pub note: Option<String>,
    pub created_at: String,
}

fn normalize_kind(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "block" => Some("block"),
        "allow" => Some("allow"),
        _ => None,
    }
}

fn normalize_scope(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "address" => Some("address"),
        "domain" => Some("domain"),
        _ => None,
    }
}

pub(crate) fn address_of(sender: &str) -> String {
    let text = sender.trim();
    if let Some(start) = text.rfind('<') {
        if let Some(end) = text[start..].find('>') {
            return text[start + 1..start + end]
                .trim()
                .trim_matches('"')
                .to_ascii_lowercase();
        }
    }
    text.trim_matches(|character: char| {
        character == '<'
            || character == '>'
            || character == '"'
            || character == ','
            || character.is_whitespace()
    })
    .to_ascii_lowercase()
}

pub(crate) fn domain_of(address: &str) -> Option<String> {
    address
        .rsplit_once('@')
        .map(|(_, domain)| domain.trim_matches('.').to_ascii_lowercase())
        .filter(|domain| !domain.is_empty())
}

fn normalize_value(scope: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Enter an address or domain".to_string());
    }
    if scope == "domain" {
        let domain = trimmed.trim_start_matches('@').trim_matches('.').to_ascii_lowercase();
        if domain.is_empty() || domain.contains('@') || domain.split_whitespace().count() != 1 {
            return Err("Enter a domain like example.com".to_string());
        }
        if !domain.contains('.') {
            return Err("Domains need at least one dot, like example.com".to_string());
        }
        return Ok(domain);
    }
    let address = address_of(trimmed);
    if !address.contains('@') || address.split_whitespace().count() != 1 {
        return Err("Enter a full email address like name@example.com".to_string());
    }
    Ok(address)
}

fn rule_matches(rule: &SenderRule, address: &str, domain: Option<&str>) -> bool {
    if rule.scope == "address" {
        return rule.value.as_str() == address;
    }
    match domain {
        Some(domain) => {
            domain == rule.value.as_str()
                || domain.ends_with(&format!(".{}", rule.value))
        }
        None => false,
    }
}

fn load_rules(database: &Connection) -> Result<Vec<SenderRule>, String> {
    let mut statement = database
        .prepare(
            "SELECT id, kind, scope, value, note, created_at FROM sender_rules
             ORDER BY kind ASC, scope ASC, value ASC",
        )
        .map_err(|error| error.to_string())?;
    let rules = statement
        .query_map([], |row| {
            Ok(SenderRule {
                id: row.get(0)?,
                kind: row.get(1)?,
                scope: row.get(2)?,
                value: row.get(3)?,
                note: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(rules)
}

/// Evaluate the allowlist and blocklist for one sender identity. Allow rules
/// always win, so `block` is `None` whenever `allowlisted` is true.
pub(crate) fn rules_for(
    database: &Connection,
    sender: &str,
) -> Result<(bool, Option<SenderRule>), String> {
    let address = address_of(sender);
    let domain = domain_of(&address);
    let rules = load_rules(database)?;
    let allowlisted = rules.iter().any(|rule| {
        rule.kind == "allow" && rule_matches(rule, &address, domain.as_deref())
    });
    let block = if allowlisted {
        None
    } else {
        rules.into_iter().find(|rule| {
            rule.kind == "block" && rule_matches(rule, &address, domain.as_deref())
        })
    };
    Ok((allowlisted, block))
}

fn matching_rule(database: &Connection, sender: &str) -> Result<Option<SenderRule>, String> {
    Ok(rules_for(database, sender)?.1)
}

fn blocked_action(database: &Connection) -> String {
    database
        .query_row(
            "SELECT value FROM settings WHERE key = 'blocked_sender_action'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| value.as_str() == "trash")
        .unwrap_or_else(|| "spam".to_string())
}

/// Re-evaluate one message against the current rules. Returns whether the
/// message is blocked.
pub(crate) fn apply_to_email(database: &Connection, email_id: i64) -> Result<bool, String> {
    let message = database
        .query_row(
            "SELECT e.sender, e.sender_address, f.name, e.blocked_by
             FROM emails e JOIN folders f ON f.id = e.folder_id
             WHERE e.id = ?1 AND e.deleted_at IS NULL",
            params![email_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((sender, sender_address, folder, blocked_by)) = message else {
        return Ok(false);
    };

    // Only incoming mail is filtered. Sent, draft and archived messages keep
    // their folder, and existing Spam/Trash messages are only re-evaluated when
    // an earlier rule put them there.
    let folder_key = folder.to_ascii_lowercase();
    let filtered_folder = folder_key == "spam" || folder_key == "trash";
    if !matches!(folder_key.as_str(), "inbox" | "snoozed" | "spam" | "trash") {
        return Ok(false);
    }
    if filtered_folder && blocked_by.is_none() {
        return Ok(false);
    }

    let identity = if sender_address.trim().is_empty() {
        sender.as_str()
    } else {
        sender_address.as_str()
    };

    match matching_rule(database, identity)? {
        Some(rule) => {
            let folder = if blocked_action(database) == "trash" {
                "Trash"
            } else {
                "Spam"
            };
            let folder_id = mailbox::folder_id(database, folder)?;
            database
                .execute(
                    "UPDATE emails SET folder_id = ?2, blocked_by = ?3,
                        updated_at = datetime('now')
                     WHERE id = ?1",
                    params![
                        email_id,
                        folder_id,
                        format!("{}:{}", rule.scope, rule.value)
                    ],
                )
                .map_err(|error| error.to_string())?;
            Ok(true)
        }
        None => {
            if blocked_by.is_some() {
                let inbox_id = mailbox::folder_id(database, "Inbox")?;
                database
                    .execute(
                        "UPDATE emails
                         SET blocked_by = NULL,
                             folder_id = CASE
                                 WHEN lower((SELECT name FROM folders WHERE id = folder_id))
                                      IN ('spam', 'trash')
                                 THEN ?2 ELSE folder_id END,
                             updated_at = datetime('now')
                         WHERE id = ?1",
                        params![email_id, inbox_id],
                    )
                    .map_err(|error| error.to_string())?;
            }
            Ok(false)
        }
    }
}

fn apply_all(database: &Connection) -> Result<i64, String> {
    let mut statement = database
        .prepare("SELECT id FROM emails WHERE deleted_at IS NULL")
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    drop(statement);

    let mut blocked = 0;
    for id in ids {
        if apply_to_email(database, id)? {
            blocked += 1;
        }
    }
    Ok(blocked)
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS sender_rules (
                id INTEGER PRIMARY KEY,
                kind TEXT NOT NULL CHECK(kind IN ('block', 'allow')),
                scope TEXT NOT NULL CHECK(scope IN ('address', 'domain')),
                value TEXT NOT NULL,
                note TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(kind, scope, value)
            );
            CREATE INDEX IF NOT EXISTS sender_rules_lookup
                ON sender_rules(kind, scope, value);",
        )
        .map_err(|error| error.to_string())?;
    database
        .execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('blocked_sender_action', 'spam')",
            [],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_sender_rules(app: AppHandle) -> Result<Vec<SenderRule>, String> {
    let database = storage::connection(&app)?;
    load_rules(&database)
}

#[tauri::command]
pub fn save_sender_rule(
    app: AppHandle,
    kind: String,
    scope: String,
    value: String,
    note: Option<String>,
) -> Result<SenderRule, String> {
    let database = storage::connection(&app)?;
    let kind = normalize_kind(&kind).ok_or_else(|| "Rule type must be block or allow".to_string())?;
    let scope =
        normalize_scope(&scope).ok_or_else(|| "Rule scope must be address or domain".to_string())?;
    let value = normalize_value(scope, &value)?;
    let note = note
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());

    database
        .execute(
            "INSERT INTO sender_rules (kind, scope, value, note)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(kind, scope, value) DO UPDATE SET note = excluded.note",
            params![kind, scope, value, note],
        )
        .map_err(|error| error.to_string())?;

    let rule = database
        .query_row(
            "SELECT id, kind, scope, value, note, created_at FROM sender_rules
             WHERE kind = ?1 AND scope = ?2 AND value = ?3",
            params![kind, scope, value],
            |row| {
                Ok(SenderRule {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    scope: row.get(2)?,
                    value: row.get(3)?,
                    note: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .map_err(|error| error.to_string())?;

    apply_all(&database)?;
    Ok(rule)
}

#[tauri::command]
pub fn delete_sender_rule(app: AppHandle, id: i64) -> Result<bool, String> {
    let database = storage::connection(&app)?;
    let changed = database
        .execute("DELETE FROM sender_rules WHERE id = ?1", params![id])
        .map_err(|error| error.to_string())?;
    if changed > 0 {
        apply_all(&database)?;
    }
    Ok(changed > 0)
}

#[tauri::command]
pub fn apply_sender_rules(app: AppHandle) -> Result<i64, String> {
    let database = storage::connection(&app)?;
    apply_all(&database)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(kind: &str, scope: &str, value: &str) -> SenderRule {
        SenderRule {
            id: 1,
            kind: kind.to_string(),
            scope: scope.to_string(),
            value: value.to_string(),
            note: None,
            created_at: String::new(),
        }
    }

    #[test]
    fn extracts_addresses_from_display_names() {
        assert_eq!(address_of("Maya Chen <Maya@Example.com>"), "maya@example.com");
        assert_eq!(address_of("noah@example.com"), "noah@example.com");
        assert_eq!(domain_of("maya@example.com").as_deref(), Some("example.com"));
    }

    #[test]
    fn matches_address_and_subdomains() {
        let address = rule("block", "address", "spam@example.com");
        assert!(rule_matches(&address, "spam@example.com", Some("example.com")));
        assert!(!rule_matches(&address, "other@example.com", Some("example.com")));

        let domain = rule("block", "domain", "example.com");
        assert!(rule_matches(&domain, "a@example.com", Some("example.com")));
        assert!(rule_matches(&domain, "a@mail.example.com", Some("mail.example.com")));
        assert!(!rule_matches(&domain, "a@example.org", Some("example.org")));
    }

    #[test]
    fn validates_rule_values() {
        assert_eq!(normalize_value("domain", "@Example.COM").unwrap(), "example.com");
        assert_eq!(
            normalize_value("address", "Name <user@example.com>").unwrap(),
            "user@example.com"
        );
        assert!(normalize_value("domain", "not a domain").is_err());
        assert!(normalize_value("address", "user").is_err());
    }
}
