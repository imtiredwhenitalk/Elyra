//! Sender-domain reputation classification.
//!
//! Reputation combines local authentication results (SPF, DKIM, DMARC),
//! allow/block rules, brand-impersonation checks, and any threat-intelligence
//! verdicts already cached for the message. Unknown domains start at Neutral
//! and are never treated as malicious without evidence. Results are cached in
//! SQLite and can be refreshed when new intelligence arrives.

use std::collections::HashMap;
use std::sync::OnceLock;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::threats::UrlScan;
use crate::{mailbox, security, senders, shield, storage};

const SUSPICIOUS_THRESHOLD: i64 = 25;
const MALICIOUS_THRESHOLD: i64 = 60;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AuthResults {
    pub spf: Option<String>,
    pub dkim: Option<String>,
    pub dmarc: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SenderReputation {
    pub email_id: i64,
    pub domain: String,
    pub address: Option<String>,
    pub classification: String,
    pub score: i64,
    pub reasons: Vec<String>,
    pub auth: AuthResults,
    pub allowlisted: bool,
    pub blocklisted: bool,
    pub disputed: bool,
    pub feedback_note: Option<String>,
    pub updated_at: String,
}

struct Evaluation {
    domain: String,
    address: Option<String>,
    score: i64,
    reasons: Vec<String>,
    auth: AuthResults,
    allowlisted: bool,
    blocklisted: bool,
}

struct StoredReputation {
    disputed: bool,
    dismissed_score: i64,
    feedback_note: Option<String>,
}

fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn classification_for(score: i64) -> &'static str {
    if score >= MALICIOUS_THRESHOLD {
        "malicious"
    } else if score >= SUSPICIOUS_THRESHOLD {
        "suspicious"
    } else {
        "neutral"
    }
}

fn auth_pattern() -> &'static regex::Regex {
    static PATTERN: OnceLock<regex::Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::Regex::new(r"(?i)\b(spf|dkim|dmarc)\s*=\s*([a-z]+)").expect("valid auth pattern")
    })
}

/// Read one header value, following folded continuation lines. Only the header
/// block is considered, so message bodies cannot spoof authentication results.
fn header_value(text: &str, name: &str) -> Option<String> {
    let mut value: Option<String> = None;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            break;
        }
        if let Some(current) = value.as_mut() {
            if line.starts_with(' ') || line.starts_with('\t') {
                current.push(' ');
                current.push_str(line.trim());
                continue;
            }
            break;
        }
        if let Some((key, rest)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case(name) {
                value = Some(rest.trim().to_string());
            }
        }
    }
    value
}

/// Parse `Authentication-Results` (or `Received-SPF`) from raw message bytes.
/// Returns `None` when neither header is present.
pub(crate) fn parse_auth_results(raw: &[u8]) -> Option<AuthResults> {
    let text = String::from_utf8_lossy(raw);
    if let Some(header) = header_value(&text, "authentication-results") {
        let mut results = AuthResults {
            source: Some("Authentication-Results".to_string()),
            ..AuthResults::default()
        };
        for captures in auth_pattern().captures_iter(&header) {
            let Some(key) = captures.get(1).map(|value| value.as_str().to_ascii_lowercase()) else {
                continue;
            };
            let Some(value) = captures.get(2).map(|value| value.as_str().to_ascii_lowercase()) else {
                continue;
            };
            match key.as_str() {
                "spf" => results.spf = Some(value),
                "dkim" => results.dkim = Some(value),
                "dmarc" => results.dmarc = Some(value),
                _ => {}
            }
        }
        return Some(results);
    }

    let header = header_value(&text, "received-spf")?;
    let first = header
        .split_whitespace()
        .next()
        .map(|value| value.trim_matches(|character: char| !character.is_ascii_alphabetic()).to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    Some(AuthResults {
        spf: first,
        source: Some("Received-SPF".to_string()),
        ..AuthResults::default()
    })
}

fn auth_for_email(
    database: &Connection,
    email_id: i64,
) -> Result<(String, String, AuthResults), String> {
    let row = database
        .query_row(
            "SELECT sender, sender_address, auth_results
             FROM emails WHERE id = ?1 AND deleted_at IS NULL",
            params![email_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Message not found".to_string())?;
    let auth = if row.2.trim().is_empty() {
        AuthResults::default()
    } else {
        serde_json::from_str(&row.2).unwrap_or_default()
    };
    Ok((row.0, row.1, auth))
}

fn normalized(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

fn has_auth(auth: &AuthResults) -> bool {
    normalized(&auth.spf).is_some()
        || normalized(&auth.dkim).is_some()
        || normalized(&auth.dmarc).is_some()
}

fn apply_auth(auth: &AuthResults, score: &mut i64, reasons: &mut Vec<String>) {
    if let Some(spf) = normalized(&auth.spf) {
        match spf.as_str() {
            "fail" => {
                *score += 18;
                reasons.push(
                    "SPF failed: the sending server is not authorized for this domain.".to_string(),
                );
            }
            "softfail" | "neutral" | "permerror" | "temperror" => {
                *score += 8;
                reasons.push(format!(
                    "SPF did not pass ({spf}), so the sender could not be fully verified."
                ));
            }
            _ => {}
        }
    }
    if let Some(dkim) = normalized(&auth.dkim) {
        if dkim == "fail" {
            *score += 12;
            reasons.push("DKIM failed: the message signature did not verify.".to_string());
        }
    }
    if let Some(dmarc) = normalized(&auth.dmarc) {
        match dmarc.as_str() {
            "fail" => {
                *score += 22;
                reasons.push(
                    "DMARC failed: the message does not align with the domain's published policy."
                        .to_string(),
                );
            }
            "none" => reasons.push(
                "The domain has no DMARC policy, so spoofing protection is limited.".to_string(),
            ),
            "softfail" | "permerror" | "temperror" => {
                *score += 8;
                reasons.push(format!("DMARC did not pass ({dmarc})."));
            }
            _ => {}
        }
    }
}

fn host_matches(host: &str, domain: &str) -> bool {
    !host.is_empty()
        && !domain.is_empty()
        && (host == domain || host.ends_with(&format!(".{domain}")))
}

fn apply_threat_intel(
    database: &Connection,
    email_id: i64,
    domain: &str,
    score: &mut i64,
    reasons: &mut Vec<String>,
) -> Result<(), String> {
    let urls = database
        .query_row(
            "SELECT urls FROM message_security WHERE email_id = ?1",
            params![email_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some(urls) = urls else {
        return Ok(());
    };
    let scans: Vec<UrlScan> = serde_json::from_str(&urls).unwrap_or_default();
    let mut malicious = false;
    let mut suspicious = false;
    for scan in scans {
        if !host_matches(&scan.host.to_ascii_lowercase(), domain) {
            continue;
        }
        match scan.verdict.to_ascii_lowercase().as_str() {
            "malicious" => malicious = true,
            "suspicious" => suspicious = true,
            _ => {}
        }
    }
    if malicious {
        *score += 60;
        reasons.push(format!(
            "Threat intelligence flagged a link on {domain} as malicious."
        ));
    } else if suspicious {
        *score += 30;
        reasons.push(format!(
            "Threat intelligence marked a link on {domain} as suspicious."
        ));
    }
    Ok(())
}

fn evaluate(database: &Connection, email_id: i64) -> Result<Evaluation, String> {
    let (sender, stored_address, auth) = auth_for_email(database, email_id)?;
    let identity = if stored_address.trim().is_empty() {
        sender.as_str()
    } else {
        stored_address.as_str()
    };
    let address = senders::address_of(identity);
    let domain = senders::domain_of(&address).unwrap_or_default();
    let (allowlisted, block_rule) = senders::rules_for(database, &address)?;

    let mut score = 0_i64;
    let mut reasons = Vec::new();

    if allowlisted {
        reasons.push("You allowlisted this sender, so it is treated as trusted.".to_string());
    } else if let Some(brand) = shield::known_brand(&domain) {
        reasons.push(format!("{domain} is an official {brand} domain."));
    } else {
        if let Some(rule) = block_rule.as_ref() {
            score += 20;
            reasons.push(format!(
                "You blocked this sender {} ({}).",
                if rule.scope == "domain" { "domain" } else { "address" },
                rule.value
            ));
        }
        if let Some(reason) = shield::brand_impersonation(&sender, Some(&address)) {
            score += 28;
            reasons.push(reason);
        } else if domain.starts_with("xn--") || domain.contains(".xn--") {
            score += 20;
            reasons.push(format!(
                "The sender domain {domain} uses punycode, which is often used to imitate a trusted brand."
            ));
        }
        apply_auth(&auth, &mut score, &mut reasons);
        apply_threat_intel(database, email_id, &domain, &mut score, &mut reasons)?;
        if reasons.is_empty() {
            if has_auth(&auth) {
                reasons.push("SPF, DKIM and DMARC did not report any problems.".to_string());
            } else {
                reasons.push(
                    "No SPF, DKIM, or DMARC results were available for this domain.".to_string(),
                );
            }
        }
    }

    Ok(Evaluation {
        domain,
        address: if address.is_empty() {
            None
        } else {
            Some(address)
        },
        score,
        reasons,
        auth,
        allowlisted,
        blocklisted: block_rule.is_some(),
    })
}

fn load_stored(database: &Connection, domain: &str) -> Result<Option<StoredReputation>, String> {
    database
        .query_row(
            "SELECT disputed, dismissed_score, feedback_note
             FROM domain_reputation WHERE domain = ?1",
            params![domain],
            |row| {
                Ok(StoredReputation {
                    disputed: row.get::<_, i64>(0)? != 0,
                    dismissed_score: row.get(1)?,
                    feedback_note: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn store_reputation(
    database: &Connection,
    domain: &str,
    classification: &str,
    score: i64,
    reasons: &[String],
    auth: &AuthResults,
    disputed: bool,
    dismissed_score: i64,
    feedback_note: Option<&str>,
    updated_at: &str,
) -> Result<(), String> {
    let reasons_json = serde_json::to_string(reasons).unwrap_or_else(|_| "[]".to_string());
    let auth_json = serde_json::to_string(auth).unwrap_or_else(|_| "{}".to_string());
    database
        .execute(
            "INSERT INTO domain_reputation
                (domain, classification, score, reasons, auth, disputed, dismissed_score,
                 feedback_note, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(domain) DO UPDATE SET
                classification = excluded.classification,
                score = excluded.score,
                reasons = excluded.reasons,
                auth = excluded.auth,
                disputed = excluded.disputed,
                dismissed_score = excluded.dismissed_score,
                feedback_note = excluded.feedback_note,
                updated_at = excluded.updated_at",
            params![
                domain,
                classification,
                score,
                reasons_json,
                auth_json,
                if disputed { 1 } else { 0 },
                dismissed_score,
                feedback_note,
                updated_at
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn finalize(
    database: &Connection,
    email_id: i64,
    evaluation: Evaluation,
) -> Result<SenderReputation, String> {
    let Evaluation {
        domain,
        address,
        score: raw_score,
        reasons: raw_reasons,
        auth,
        allowlisted,
        blocklisted,
    } = evaluation;
    let stored = if domain.is_empty() {
        None
    } else {
        load_stored(database, &domain)?
    };

    let mut classification = classification_for(raw_score).to_string();
    let mut score = raw_score;
    let mut reasons = raw_reasons;
    let mut disputed = stored.as_ref().map(|item| item.disputed).unwrap_or(false);
    let mut dismissed_score = stored
        .as_ref()
        .map(|item| item.dismissed_score)
        .unwrap_or(0);
    let feedback_note = stored.and_then(|item| item.feedback_note);

    if disputed {
        if score <= dismissed_score {
            classification = "neutral".to_string();
            score = dismissed_score;
            reasons = vec![
                "You reported this assessment as incorrect. Elyra is showing it as Neutral until new intelligence arrives."
                    .to_string(),
            ];
        } else {
            disputed = false;
            dismissed_score = 0;
            reasons.push("New intelligence changed this assessment after your report.".to_string());
        }
    }

    let updated_at = now_string();
    if !domain.is_empty() {
        store_reputation(
            database,
            &domain,
            &classification,
            score,
            &reasons,
            &auth,
            disputed,
            dismissed_score,
            feedback_note.as_deref(),
            &updated_at,
        )?;
    }

    Ok(SenderReputation {
        email_id,
        domain,
        address,
        classification,
        score,
        reasons,
        auth,
        allowlisted,
        blocklisted,
        disputed,
        feedback_note,
        updated_at,
    })
}

fn compute_reputation(
    database: &Connection,
    email_id: i64,
) -> Result<SenderReputation, String> {
    let evaluation = evaluate(database, email_id)?;
    finalize(database, email_id, evaluation)
}

/// Refresh the cached reputation for one message. Used after a link scan adds
/// new threat-intelligence results.
pub(crate) fn refresh_for_email(database: &Connection, email_id: i64) -> Result<(), String> {
    compute_reputation(database, email_id)?;
    Ok(())
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    mailbox::ensure_column(
        &database,
        "emails",
        "auth_results",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS domain_reputation (
                domain TEXT PRIMARY KEY,
                classification TEXT NOT NULL DEFAULT 'neutral',
                score INTEGER NOT NULL DEFAULT 0,
                reasons TEXT NOT NULL DEFAULT '[]',
                auth TEXT NOT NULL DEFAULT '{}',
                disputed INTEGER NOT NULL DEFAULT 0,
                dismissed_score INTEGER NOT NULL DEFAULT 0,
                feedback_note TEXT,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS reputation_feedback (
                id INTEGER PRIMARY KEY,
                domain TEXT NOT NULL,
                email_id INTEGER,
                note TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_reputation_feedback_domain
                ON reputation_feedback(domain);",
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_sender_reputation(app: AppHandle, id: i64) -> Result<SenderReputation, String> {
    let database = storage::connection(&app)?;
    compute_reputation(&database, id)
}

#[tauri::command]
pub fn refresh_domain_reputation(
    app: AppHandle,
    domain: Option<String>,
) -> Result<i64, String> {
    let database = storage::connection(&app)?;
    let target = domain
        .map(|value| {
            value
                .trim()
                .trim_start_matches('@')
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty());

    let mut statement = database
        .prepare("SELECT id FROM emails WHERE deleted_at IS NULL")
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    drop(statement);

    let mut best: HashMap<String, Evaluation> = HashMap::new();
    for id in ids {
        let evaluation = evaluate(&database, id)?;
        if evaluation.domain.is_empty() {
            continue;
        }
        if let Some(target) = target.as_deref() {
            if evaluation.domain != target {
                continue;
            }
        }
        match best.get_mut(&evaluation.domain) {
            Some(existing) if evaluation.score > existing.score => {
                existing.score = evaluation.score;
                existing.reasons = evaluation.reasons;
                existing.auth = evaluation.auth;
            }
            Some(existing) => {
                for reason in evaluation.reasons {
                    if !existing.reasons.contains(&reason) {
                        existing.reasons.push(reason);
                    }
                }
            }
            None => {
                best.insert(evaluation.domain.clone(), evaluation);
            }
        }
    }

    let refreshed = best.len() as i64;
    for (_, evaluation) in best {
        finalize(&database, 0, evaluation)?;
    }
    security::record_event(
        &database,
        "reputation.refresh",
        "success",
        &format!("Refreshed {refreshed} sender domain reputations"),
    )?;
    Ok(refreshed)
}

#[tauri::command]
pub fn report_reputation_issue(
    app: AppHandle,
    id: i64,
    note: Option<String>,
) -> Result<SenderReputation, String> {
    let database = storage::connection(&app)?;
    let reputation = compute_reputation(&database, id)?;
    if reputation.domain.is_empty() {
        return Err("This message has no sender domain to assess.".to_string());
    }
    let note = note
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let now = now_string();
    database
        .execute(
            "UPDATE domain_reputation
             SET disputed = 1, dismissed_score = ?2, feedback_note = ?3,
                 classification = 'neutral', updated_at = ?4
             WHERE domain = ?1",
            params![reputation.domain, reputation.score, note, now],
        )
        .map_err(|error| error.to_string())?;
    database
        .execute(
            "INSERT INTO reputation_feedback (domain, email_id, note, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![reputation.domain, id, note, now],
        )
        .map_err(|error| error.to_string())?;
    security::record_event(
        &database,
        "reputation.feedback",
        "success",
        &format!(
            "Sender domain {} reported as an incorrect assessment",
            reputation.domain
        ),
    )?;
    compute_reputation(&database, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_authentication_results_headers() {
        let raw = b"From: sender@example.com\r\nAuthentication-Results: mx.example.net;\r\n spf=pass smtp.mailfrom=example.com;\r\n dkim=fail header.d=example.com;\r\n dmarc=pass\r\nSubject: Hello\r\n\r\nspf=fail dkim=fail dmarc=fail";
        let auth = parse_auth_results(raw).expect("auth header");
        assert_eq!(auth.spf.as_deref(), Some("pass"));
        assert_eq!(auth.dkim.as_deref(), Some("fail"));
        assert_eq!(auth.dmarc.as_deref(), Some("pass"));
        assert_eq!(auth.source.as_deref(), Some("Authentication-Results"));
    }

    #[test]
    fn ignores_body_text_when_headers_are_missing() {
        let raw = b"From: sender@example.com\r\nSubject: Hello\r\n\r\nspf=pass dkim=pass dmarc=pass";
        assert!(parse_auth_results(raw).is_none());
    }

    #[test]
    fn reads_received_spf_fallback() {
        let raw = b"Received-SPF: Pass (mailfrom) identity=mailfrom; client-ip=192.0.2.1\r\nSubject: Hello\r\n\r\nBody";
        let auth = parse_auth_results(raw).expect("received-spf header");
        assert_eq!(auth.spf.as_deref(), Some("pass"));
        assert_eq!(auth.source.as_deref(), Some("Received-SPF"));
    }

    #[test]
    fn classifies_scores_conservatively() {
        assert_eq!(classification_for(0), "neutral");
        assert_eq!(classification_for(24), "neutral");
        assert_eq!(classification_for(25), "suspicious");
        assert_eq!(classification_for(59), "suspicious");
        assert_eq!(classification_for(60), "malicious");
    }

    #[test]
    fn matches_sender_domain_and_subdomains() {
        assert!(host_matches("example.com", "example.com"));
        assert!(host_matches("mail.example.com", "example.com"));
        assert!(!host_matches("example.org", "example.com"));
        assert!(!host_matches("notexample.com", "example.com"));
    }

    #[test]
    fn auth_failures_add_explanations() {
        let auth = AuthResults {
            spf: Some("fail".to_string()),
            dkim: Some("fail".to_string()),
            dmarc: Some("fail".to_string()),
            source: Some("test".to_string()),
        };
        let mut score = 0;
        let mut reasons = Vec::new();
        apply_auth(&auth, &mut score, &mut reasons);
        assert_eq!(score, 52);
        assert_eq!(reasons.len(), 3);
    }
}
