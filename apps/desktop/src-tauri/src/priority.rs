//! Message importance and priority suggestions.
//!
//! Priority is a local, deterministic signal: Low, Normal, High or Urgent.
//! Suggestions are applied when mail is imported, unless the user turns
//! automatic suggestions off in Settings. Manual changes always win because
//! they set `priority_source = 'manual'` and are never recomputed.

use rusqlite::{params, Connection, OptionalExtension};
use tauri::AppHandle;

use crate::storage;

const URGENT_SIGNALS: &[&str] = &[
    "urgent",
    "asap",
    "action required",
    "immediate action",
    "time sensitive",
    "final notice",
    "deadline today",
    "respond today",
];

const HIGH_SIGNALS: &[&str] = &[
    "invoice",
    "payment",
    "contract",
    "proposal",
    "approval",
    "review",
    "deadline",
    "security alert",
    "sign-in",
    "password",
    "interview",
    "offer",
    "escalation",
    "needs your",
    "please confirm",
    "by tomorrow",
];

const LOW_SIGNALS: &[&str] = &[
    "newsletter",
    "digest",
    "unsubscribe",
    "promotion",
    "weekly update",
    "automated",
    "notification",
    "no-reply",
    "noreply",
];

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub fn normalize_priority(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "low" => Some("low"),
        "normal" => Some("normal"),
        "high" => Some("high"),
        "urgent" => Some("urgent"),
        _ => None,
    }
}

fn setting(database: &Connection, key: &str) -> Option<String> {
    database
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
}

pub fn auto_priority_enabled(database: &Connection) -> bool {
    setting(database, "auto_priority")
        .map(|value| !value.eq_ignore_ascii_case("false"))
        .unwrap_or(true)
}

pub fn default_priority(database: &Connection) -> String {
    setting(database, "priority_default")
        .and_then(|value| normalize_priority(&value).map(str::to_string))
        .unwrap_or_else(|| "normal".to_string())
}

/// Return `(priority, important suggestion, reason)` for a message.
pub(crate) fn suggestion_for(
    sender: &str,
    subject: &str,
    body: &str,
    tag: &str,
) -> (String, bool, Option<String>) {
    let sender = sender.to_ascii_lowercase();
    let subject = subject.to_ascii_lowercase();
    let body = body.to_ascii_lowercase();
    let tag = tag.to_ascii_lowercase();

    if tag == "spam"
        || tag == "trash"
        || sender.contains("no-reply")
        || sender.contains("noreply")
        || contains_any(&subject, LOW_SIGNALS)
    {
        return (
            "low".to_string(),
            false,
            Some("Looks like an automated or low-priority update.".to_string()),
        );
    }

    if contains_any(&subject, URGENT_SIGNALS) {
        return (
            "urgent".to_string(),
            true,
            Some("The subject asks for urgent action.".to_string()),
        );
    }

    let important_tag = tag == "important" || tag == "security";
    if important_tag
        || contains_any(&subject, HIGH_SIGNALS)
        || contains_any(&body, &["by tomorrow", "needs your approval", "please review"])
    {
        return (
            "high".to_string(),
            important_tag,
            Some("Matches important or time-sensitive signals.".to_string()),
        );
    }

    (
        "normal".to_string(),
        false,
        Some("No strong priority signals detected.".to_string()),
    )
}

pub(crate) fn apply_auto_priority(
    database: &Connection,
    email_id: i64,
    suggest_importance: bool,
) -> Result<(), String> {
    let row = database
        .query_row(
            "SELECT sender, subject, body_text, tag FROM emails
             WHERE id = ?1 AND deleted_at IS NULL",
            params![email_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((sender, subject, body, tag)) = row else {
        return Ok(());
    };

    let (priority, important, reason) = if auto_priority_enabled(database) {
        suggestion_for(&sender, &subject, &body, &tag)
    } else {
        (default_priority(database), false, None)
    };

    database
        .execute(
            "UPDATE emails SET priority = ?2, priority_source = 'auto',
                priority_reason = ?3,
                is_important = CASE WHEN ?4 = 1 THEN 1 ELSE is_important END,
                updated_at = datetime('now')
             WHERE id = ?1 AND priority_source = 'auto'",
            params![
                email_id,
                priority,
                reason,
                if suggest_importance && important { 1 } else { 0 }
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    for (key, value) in [("auto_priority", "true"), ("priority_default", "normal")] {
        database
            .execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Recompute every automatic priority without touching manual choices or the
/// important flag. Used after the preference changes or the rules are updated.
#[tauri::command]
pub fn refresh_priority_suggestions(app: AppHandle) -> Result<i64, String> {
    let database = storage::connection(&app)?;
    let mut statement = database
        .prepare("SELECT id FROM emails WHERE deleted_at IS NULL AND priority_source = 'auto'")
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    for id in &ids {
        apply_auto_priority(&database, *id, false)?;
    }
    Ok(ids.len() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_known_priorities() {
        assert_eq!(normalize_priority("URGENT"), Some("urgent"));
        assert_eq!(normalize_priority("normal"), Some("normal"));
        assert_eq!(normalize_priority("later"), None);
    }

    #[test]
    fn detects_urgent_subjects() {
        let (priority, important, _) =
            suggestion_for("Maya Chen", "Urgent: sign the contract today", "", "Personal");
        assert_eq!(priority, "urgent");
        assert!(important);
    }

    #[test]
    fn keeps_automated_mail_low() {
        let (priority, important, _) = suggestion_for(
            "Linear",
            "Your weekly product digest",
            "Unsubscribe from these updates.",
            "Updates",
        );
        assert_eq!(priority, "low");
        assert!(!important);
    }

    #[test]
    fn leaves_ordinary_mail_normal() {
        let (priority, important, _) =
            suggestion_for("Noah Williams", "Coffee next week?", "No agenda.", "Personal");
        assert_eq!(priority, "normal");
        assert!(!important);
    }
}
