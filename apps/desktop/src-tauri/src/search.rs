//! Advanced, local-only mailbox search.
//!
//! Every filter is applied by SQLite against the local database. No query text,
//! sender address or message content is sent to an external service.

use chrono::NaiveDate;
use rusqlite::{params_from_iter, types::Value};
use tauri::AppHandle;

use crate::{mailbox, storage};

const PRIORITIES: &[&str] = &["low", "normal", "high", "urgent"];
const SECURITY_STATES: &[&str] = &["safe", "suspicious", "malicious", "unscanned"];
const REPUTATIONS: &[&str] = &["trusted", "neutral", "suspicious", "malicious"];

fn like_pattern(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

fn allowed(values: &Option<Vec<String>>, options: &[&str]) -> Vec<String> {
    values
        .clone()
        .unwrap_or_default()
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| options.contains(&value.as_str()))
        .collect::<Vec<_>>()
}

fn in_condition(column: &str, values: &[String], params: &mut Vec<Value>) -> String {
    for value in values {
        params.push(Value::Text(value.clone()));
    }
    format!("{column} IN ({})", vec!["?"; values.len()].join(", "))
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn advanced_search(
    app: AppHandle,
    query: Option<String>,
    from: Option<String>,
    to: Option<String>,
    subject: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    has_attachments: Option<bool>,
    important: Option<bool>,
    priorities: Option<Vec<String>>,
    security: Option<Vec<String>>,
    reputation: Option<Vec<String>>,
    folder: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<mailbox::MailboxPage, String> {
    let database = storage::connection(&app)?;
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let offset = offset.unwrap_or(0).max(0);

    let mut conditions = vec!["e.deleted_at IS NULL".to_string()];
    let mut values: Vec<Value> = Vec::new();

    let mut add_like = |column: &str, value: &Option<String>| {
        if let Some(text) = value {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                conditions.push(format!("{column} LIKE ? ESCAPE '\\'"));
                values.push(Value::Text(like_pattern(trimmed)));
            }
        }
    };

    add_like("e.sender", &from);
    add_like("e.recipient", &to);
    add_like("e.subject", &subject);

    if let Some(text) = query
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        conditions.push(
            "(e.sender LIKE ? ESCAPE '\\' OR e.recipient LIKE ? ESCAPE '\\' \
              OR e.subject LIKE ? ESCAPE '\\' OR e.preview LIKE ? ESCAPE '\\' \
              OR e.body_text LIKE ? ESCAPE '\\')"
                .to_string(),
        );
        for _ in 0..5 {
            values.push(Value::Text(like_pattern(text)));
        }
    }

    if let Some(start) = date_from
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let parsed = NaiveDate::parse_from_str(start, "%Y-%m-%d")
            .map_err(|_| "Start date must use YYYY-MM-DD".to_string())?;
        conditions.push("e.received_at >= ?".to_string());
        values.push(Value::Text(format!(
            "{} 00:00:00",
            parsed.format("%Y-%m-%d")
        )));
    }
    if let Some(end) = date_to
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let parsed = NaiveDate::parse_from_str(end, "%Y-%m-%d")
            .map_err(|_| "End date must use YYYY-MM-DD".to_string())?;
        conditions.push("e.received_at <= ?".to_string());
        values.push(Value::Text(format!(
            "{} 23:59:59",
            parsed.format("%Y-%m-%d")
        )));
    }

    if let Some(flag) = has_attachments {
        conditions.push("e.has_attachments = ?".to_string());
        values.push(Value::Integer(i64::from(flag)));
    }
    if let Some(flag) = important {
        conditions.push("e.is_important = ?".to_string());
        values.push(Value::Integer(i64::from(flag)));
    }
    if let Some(name) = folder
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        conditions.push("lower(f.name) = ?".to_string());
        values.push(Value::Text(name.to_ascii_lowercase()));
    }

    let selected_priorities = allowed(&priorities, PRIORITIES);
    if let Some(requested) = &priorities {
        if requested.iter().any(|value| !value.trim().is_empty()) {
            if selected_priorities.is_empty() {
                conditions.push("1 = 0".to_string());
            } else {
                conditions.push(in_condition(
                    "lower(e.priority)",
                    &selected_priorities,
                    &mut values,
                ));
            }
        }
    }

    let selected_security = allowed(&security, SECURITY_STATES);
    if let Some(requested) = &security {
        if requested.iter().any(|value| !value.trim().is_empty()) {
            if selected_security.is_empty() {
                conditions.push("1 = 0".to_string());
            } else {
                conditions.push(in_condition(
                    "lower(coalesce(s.risk, 'unscanned'))",
                    &selected_security,
                    &mut values,
                ));
            }
        }
    }

    let selected_reputation = allowed(&reputation, REPUTATIONS);
    if let Some(requested) = &reputation {
        if requested.iter().any(|value| !value.trim().is_empty()) {
            if selected_reputation.is_empty() {
                conditions.push("1 = 0".to_string());
            } else {
                conditions.push(in_condition(
                    "lower(coalesce(r.classification, 'neutral'))",
                    &selected_reputation,
                    &mut values,
                ));
            }
        }
    }

    let where_clause = conditions.join(" AND ");
    let joins = " LEFT JOIN message_security s ON s.email_id = e.id \
                 LEFT JOIN domain_reputation r ON r.domain = lower(\
                     substr(coalesce(e.sender_address, e.sender), \
                            instr(coalesce(e.sender_address, e.sender), '@') + 1))";

    let mut page_values = values.clone();
    page_values.push(Value::Integer(limit));
    page_values.push(Value::Integer(offset));
    let sql = format!(
        "{}{} WHERE {where_clause} ORDER BY e.received_at DESC, e.id DESC LIMIT ? OFFSET ?",
        mailbox::summary_sql(),
        joins
    );
    let mut statement = database.prepare(&sql).map_err(|error| error.to_string())?;
    let messages = statement
        .query_map(params_from_iter(page_values), mailbox::row_to_summary)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;

    // Count with the same joins and filters, but without the row projection.
    let count_sql = format!(
        "SELECT COUNT(*) FROM emails e JOIN folders f ON f.id = e.folder_id{joins} WHERE {where_clause}"
    );
    let total: i64 = database
        .query_row(&count_sql, params_from_iter(values), |row| row.get(0))
        .map_err(|error| error.to_string())?;

    let snapshot = mailbox::stats(&app)?;
    Ok(mailbox::MailboxPage {
        has_more: offset + (messages.len() as i64) < total,
        messages,
        total,
        unread: snapshot.unread,
        starred: snapshot.starred,
        filter: "search".to_string(),
        quick_filter: "advanced".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_pattern_escapes_sql_wildcards() {
        assert_eq!(like_pattern("50%_done"), "%50\\%\\_done%");
    }

    #[test]
    fn only_known_filters_survive_validation() {
        let priorities = Some(vec!["high".to_string(), "drop table".to_string()]);
        assert_eq!(allowed(&priorities, PRIORITIES), vec!["high"]);
    }
}
