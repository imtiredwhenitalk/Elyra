//! Elyra Shield: local email content analysis plus optional threat intelligence.
//!
//! Shield runs entirely inside the Rust core. Local heuristics (sender/domain
//! checks, urgency language, credential requests, dangerous attachments and
//! visible-link mismatches) never leave the device. When the user connects a
//! threat intelligence provider, extracted URLs are checked through
//! [`crate::threats`] and the combined result is cached per message.

use std::sync::OnceLock;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::AppHandle;

use crate::threats::{self, UrlScan};
use crate::{integrations, mailbox, security, storage};

const SUSPICIOUS_THRESHOLD: i64 = 25;
const MALICIOUS_THRESHOLD: i64 = 60;

const BRANDS: &[(&str, &[&str])] = &[
    ("paypal", &["paypal.com", "paypal.me"]),
    (
        "microsoft",
        &["microsoft.com", "outlook.com", "live.com", "office.com", "microsoftonline.com"],
    ),
    ("google", &["google.com", "gmail.com"]),
    ("apple", &["apple.com", "icloud.com", "me.com"]),
    ("amazon", &["amazon.com", "amazon.co.uk", "amazon.de"]),
    ("github", &["github.com"]),
    ("netflix", &["netflix.com"]),
    ("dropbox", &["dropbox.com"]),
    ("docusign", &["docusign.com", "docusign.net"]),
    ("coinbase", &["coinbase.com"]),
];

const URGENCY_PHRASES: &[&str] = &[
    "urgent",
    "immediately",
    "verify your account",
    "account suspended",
    "account has been suspended",
    "final warning",
    "act now",
    "within 24 hours",
    "limited time",
    "click here",
    "confirm your identity",
    "unusual activity",
    "unauthorized login",
];

const CREDENTIAL_PHRASES: &[&str] = &[
    "password",
    "one-time code",
    "one time code",
    "otp",
    "social security",
    "credit card",
    "bank account",
    "wire transfer",
    "gift card",
    "seed phrase",
    "recovery code",
    "login code",
];

const DANGEROUS_EXTENSIONS: &[&str] = &[
    "exe", "scr", "js", "jse", "vbs", "vbe", "bat", "cmd", "com", "pif", "msi", "iso", "lnk",
    "hta", "jar", "ps1",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSecurity {
    pub email_id: i64,
    pub risk: String,
    pub score: i64,
    pub reasons: Vec<String>,
    pub urls: Vec<UrlScan>,
    pub external_scanned: bool,
    pub false_positive: bool,
    pub false_positive_note: Option<String>,
    pub analyzed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAlert {
    pub email_id: i64,
    pub risk: String,
    pub score: i64,
    pub reasons: Vec<String>,
    pub false_positive: bool,
    pub analyzed_at: String,
    pub sender: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShieldSummary {
    pub analyzed: i64,
    pub safe: i64,
    pub suspicious: i64,
    pub malicious: i64,
    pub false_positives: i64,
    pub providers_connected: i64,
    pub external_scanning: bool,
}

struct LocalAnalysis {
    score: i64,
    reasons: Vec<String>,
    urls: Vec<String>,
}

fn email_regex() -> &'static regex::Regex {
    static PATTERN: OnceLock<regex::Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::Regex::new(r"(?i)([a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,})")
            .expect("valid email pattern")
    })
}

fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    mailbox::ensure_column(
        &database,
        "emails",
        "sender_address",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS message_security (
                email_id INTEGER PRIMARY KEY REFERENCES emails(id) ON DELETE CASCADE,
                risk TEXT NOT NULL DEFAULT 'safe',
                score INTEGER NOT NULL DEFAULT 0,
                reasons TEXT NOT NULL DEFAULT '[]',
                urls TEXT NOT NULL DEFAULT '[]',
                external_scanned INTEGER NOT NULL DEFAULT 0,
                false_positive INTEGER NOT NULL DEFAULT 0,
                false_positive_note TEXT,
                analyzed_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_message_security_risk ON message_security(risk);",
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn setting_enabled(database: &Connection, key: &str) -> bool {
    database
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
        .map(|value| !value.eq_ignore_ascii_case("false"))
        .unwrap_or(true)
}

fn risk_for(score: i64) -> &'static str {
    if score >= MALICIOUS_THRESHOLD {
        "malicious"
    } else if score >= SUSPICIOUS_THRESHOLD {
        "suspicious"
    } else {
        "safe"
    }
}

fn sender_address(sender: &str, stored: &str) -> Option<String> {
    let stored = stored.trim();
    if !stored.is_empty() {
        return Some(stored.to_ascii_lowercase());
    }
    email_regex()
        .captures(sender)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_ascii_lowercase())
}

fn domain_of(address: &str) -> Option<String> {
    address
        .rsplit_once('@')
        .map(|(_, domain)| domain.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|domain| !domain.is_empty())
}

fn normalize_lookalike(domain: &str) -> String {
    domain
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| match character {
            '0' => 'o',
            '1' => 'l',
            '3' => 'e',
            '5' => 's',
            '7' => 't',
            other => other,
        })
        .collect::<String>()
        .replace("rn", "m")
}

fn is_official(domain: &str, official: &[&str]) -> bool {
    official
        .iter()
        .any(|candidate| domain == *candidate || domain.ends_with(&format!(".{candidate}")))
}

/// Return the brand whose official domain list contains `domain`.
pub(crate) fn known_brand(domain: &str) -> Option<&'static str> {
    BRANDS
        .iter()
        .find(|(_, official)| is_official(domain, official))
        .map(|(brand, _)| *brand)
}

pub(crate) fn brand_impersonation(display: &str, address: Option<&str>) -> Option<String> {
    let address = address?;
    let domain = domain_of(address)?;
    if domain.starts_with("xn--") || domain.contains(".xn--") {
        return Some(format!(
            "The sender domain {domain} uses punycode, which is often used to imitate a trusted brand."
        ));
    }
    let local = address.split('@').next().unwrap_or_default().to_ascii_lowercase();
    let display = display.to_ascii_lowercase();
    let normalized = normalize_lookalike(&domain);
    for &(brand, official) in BRANDS {
        if is_official(&domain, official) {
            continue;
        }
        let mentioned = display.contains(brand) || local.contains(brand);
        let lookalike = normalized.contains(brand);
        if mentioned || lookalike {
            return Some(format!(
                "The sender claims to be {brand}, but the address {address} is not on an official {brand} domain."
            ));
        }
    }
    None
}

fn attachment_reasons(database: &Connection, email_id: i64) -> Result<Vec<String>, String> {
    let mut statement = database
        .prepare("SELECT name FROM attachments WHERE email_id = ?1")
        .map_err(|error| error.to_string())?;
    let names = statement
        .query_map(params![email_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    let mut reasons = Vec::new();
    for name in names {
        let extension = name
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .unwrap_or_default();
        if DANGEROUS_EXTENSIONS.contains(&extension.as_str()) {
            reasons.push(format!(
                "The message includes an executable attachment ({name})."
            ));
            break;
        }
    }
    Ok(reasons)
}

fn local_analysis(database: &Connection, email_id: i64) -> Result<LocalAnalysis, String> {
    let row = database
        .query_row(
            "SELECT sender, sender_address, subject, body_text, body_html
             FROM emails WHERE id = ?1 AND deleted_at IS NULL",
            params![email_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Message not found".to_string())?;
    let (sender, stored_address, subject, body_text, body_html) = row;
    let address = sender_address(&sender, &stored_address);
    let haystack = format!("{subject}\n{body_text}").to_ascii_lowercase();

    let mut score = 0_i64;
    let mut reasons = Vec::new();

    if let Some(reason) = brand_impersonation(&sender, address.as_deref()) {
        score += 28;
        reasons.push(reason);
    }

    let urgency_hits = URGENCY_PHRASES
        .iter()
        .filter(|phrase| haystack.contains(**phrase))
        .count();
    if urgency_hits >= 2 {
        score += 18;
        reasons.push(
            "The message uses urgent or threatening language common in phishing campaigns."
                .to_string(),
        );
    }

    let credential_hits = CREDENTIAL_PHRASES
        .iter()
        .filter(|phrase| haystack.contains(**phrase))
        .count();
    if credential_hits > 0 {
        score += 10;
        reasons.push(
            "The message asks for credentials or payment details. Legitimate providers do not request these by email."
                .to_string(),
        );
    }

    for reason in attachment_reasons(database, email_id)? {
        score += 30;
        reasons.push(reason);
    }

    let mut urls = threats::extract_urls(&body_text);
    if let Some(html) = body_html.as_deref() {
        for url in threats::extract_urls(html) {
            if !urls.iter().any(|existing| existing == &url) {
                urls.push(url);
            }
        }
        let mut mismatches = 0;
        for (href, visible) in threats::link_pairs(html) {
            if visible.is_empty() {
                continue;
            }
            let href_host = threats::host_of(&href);
            let visible_host = threats::host_of(&visible);
            if let (Some(href_host), Some(visible_host)) = (href_host, visible_host) {
                if href_host != visible_host {
                    mismatches += 1;
                    if mismatches == 1 {
                        score += 22;
                        reasons.push(format!(
                            "A link shows {visible_host} but actually points to {href_host}."
                        ));
                    }
                }
            }
        }
    }

    Ok(LocalAnalysis {
        score,
        reasons,
        urls,
    })
}

fn load_analysis(database: &Connection, email_id: i64) -> Result<Option<MessageSecurity>, String> {
    database
        .query_row(
            "SELECT risk, score, reasons, urls, external_scanned, false_positive,
                    false_positive_note, analyzed_at
             FROM message_security WHERE email_id = ?1",
            params![email_id],
            |row| {
                Ok(MessageSecurity {
                    email_id,
                    risk: row.get(0)?,
                    score: row.get(1)?,
                    reasons: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    urls: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                    external_scanned: row.get::<_, i64>(4)? != 0,
                    false_positive: row.get::<_, i64>(5)? != 0,
                    false_positive_note: row.get(6)?,
                    analyzed_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn unscanned(urls: &[String]) -> Vec<UrlScan> {
    urls.iter()
        .map(|url| UrlScan {
            url: url.clone(),
            host: threats::host_of(url).unwrap_or_default(),
            verdict: "unscanned".to_string(),
            score: 0,
            providers: Vec::new(),
            cached: false,
            checked_at: now_string(),
        })
        .collect()
}

fn store_analysis(
    database: &Connection,
    email_id: i64,
    risk: &str,
    score: i64,
    reasons: &[String],
    urls: &[UrlScan],
    external_scanned: bool,
) -> Result<MessageSecurity, String> {
    let now = now_string();
    let reasons_json = serde_json::to_string(reasons).unwrap_or_else(|_| "[]".to_string());
    let urls_json = serde_json::to_string(urls).unwrap_or_else(|_| "[]".to_string());
    database
        .execute(
            "INSERT INTO message_security
                (email_id, risk, score, reasons, urls, external_scanned, analyzed_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(email_id) DO UPDATE SET
                risk = excluded.risk,
                score = excluded.score,
                reasons = excluded.reasons,
                urls = excluded.urls,
                external_scanned = excluded.external_scanned,
                analyzed_at = excluded.analyzed_at,
                updated_at = excluded.updated_at",
            params![
                email_id,
                risk,
                score,
                reasons_json,
                urls_json,
                if external_scanned { 1 } else { 0 },
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    load_analysis(database, email_id)?
        .ok_or_else(|| "Could not read the stored security analysis".to_string())
}

pub(crate) fn analyze_email_local(database: &Connection, email_id: i64) -> Result<(), String> {
    let analysis = local_analysis(database, email_id)?;
    let risk = risk_for(analysis.score);
    let urls = unscanned(&analysis.urls);
    store_analysis(
        database,
        email_id,
        risk,
        analysis.score,
        &analysis.reasons,
        &urls,
        false,
    )?;
    Ok(())
}

#[tauri::command(async)]
pub fn analyze_message(
    app: AppHandle,
    id: i64,
    rescan: Option<bool>,
) -> Result<MessageSecurity, String> {
    let database = storage::connection(&app)?;
    let existing = load_analysis(&database, id)?;
    let external_enabled = setting_enabled(&database, "external_scanning");
    let providers = integrations::active_providers(&app)?;
    if !rescan.unwrap_or(false) {
        if let Some(existing) = existing {
            if existing.external_scanned || !external_enabled || providers.is_empty() {
                return Ok(existing);
            }
        }
    }

    let local = local_analysis(&database, id)?;
    let mut score = local.score;
    let mut reasons = local.reasons;
    let mut urls = Vec::new();
    let mut external_scanned = local.urls.is_empty();

    if !local.urls.is_empty() {
        let report = threats::scan_urls(&app, local.urls.clone())?;
        external_scanned = report.scanned;
        if !report.scanned && !report.disabled {
            reasons.push(report.note.clone());
        }
        for scan in report.urls {
            if scan.verdict == "malicious" {
                score += 45;
                reasons.push(format!(
                    "Threat intelligence flagged {} as malicious.",
                    if scan.host.is_empty() { &scan.url } else { &scan.host }
                ));
            } else if scan.verdict == "suspicious" {
                score += 22;
                reasons.push(format!(
                    "Threat intelligence marked {} as suspicious.",
                    if scan.host.is_empty() { &scan.url } else { &scan.host }
                ));
            }
            urls.push(scan);
        }
    }

    let risk = risk_for(score);
    let analysis = store_analysis(
        &database,
        id,
        risk,
        score,
        &reasons,
        &urls,
        external_scanned,
    )?;
    security::record_event(
        &database,
        "shield.analyze",
        if risk == "safe" { "success" } else { "warning" },
        &format!("Message {id} analyzed: {risk} ({score}/100)"),
    )?;
    let _ = crate::reputation::refresh_for_email(&database, id);
    Ok(analysis)
}

#[tauri::command]
pub fn get_message_security(app: AppHandle, id: i64) -> Result<MessageSecurity, String> {
    let database = storage::connection(&app)?;
    if let Some(existing) = load_analysis(&database, id)? {
        return Ok(existing);
    }
    let analysis = local_analysis(&database, id)?;
    let risk = risk_for(analysis.score);
    let urls = unscanned(&analysis.urls);
    store_analysis(
        &database,
        id,
        risk,
        analysis.score,
        &analysis.reasons,
        &urls,
        false,
    )
}

#[tauri::command]
pub fn list_security_alerts(
    app: AppHandle,
    limit: Option<i64>,
) -> Result<Vec<SecurityAlert>, String> {
    let database = storage::connection(&app)?;
    let bounded = limit.unwrap_or(25).clamp(1, 200);
    let mut statement = database
        .prepare(
            "SELECT ms.email_id, ms.risk, ms.score, ms.reasons, ms.false_positive,
                    ms.analyzed_at, e.sender, e.subject
             FROM message_security ms
             JOIN emails e ON e.id = ms.email_id
             WHERE ms.risk IN ('suspicious', 'malicious') AND e.deleted_at IS NULL
             ORDER BY ms.score DESC, ms.analyzed_at DESC
             LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let alerts = statement
        .query_map(params![bounded], |row| {
            Ok(SecurityAlert {
                email_id: row.get(0)?,
                risk: row.get(1)?,
                score: row.get(2)?,
                reasons: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                false_positive: row.get::<_, i64>(4)? != 0,
                analyzed_at: row.get(5)?,
                sender: row.get(6)?,
                subject: row.get(7)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(alerts)
}

#[tauri::command]
pub fn get_shield_summary(app: AppHandle) -> Result<ShieldSummary, String> {
    let database = storage::connection(&app)?;
    let analyzed = database
        .query_row("SELECT COUNT(*) FROM message_security", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| error.to_string())?;
    let safe = database
        .query_row(
            "SELECT COUNT(*) FROM message_security WHERE risk = 'safe'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let suspicious = database
        .query_row(
            "SELECT COUNT(*) FROM message_security WHERE risk = 'suspicious'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let malicious = database
        .query_row(
            "SELECT COUNT(*) FROM message_security WHERE risk = 'malicious'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let false_positives = database
        .query_row(
            "SELECT COUNT(*) FROM message_security WHERE false_positive = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let providers_connected = integrations::active_providers(&app)?.len() as i64;
    Ok(ShieldSummary {
        analyzed,
        safe,
        suspicious,
        malicious,
        false_positives,
        providers_connected,
        external_scanning: setting_enabled(&database, "external_scanning"),
    })
}

#[tauri::command]
pub fn report_false_positive(
    app: AppHandle,
    id: i64,
    note: Option<String>,
) -> Result<MessageSecurity, String> {
    let database = storage::connection(&app)?;
    let note = note
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let changed = database
        .execute(
            "UPDATE message_security
             SET false_positive = 1, false_positive_note = ?2, updated_at = ?3
             WHERE email_id = ?1",
            params![id, note, now_string()],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("Analyze this message before reporting it.".to_string());
    }
    security::record_event(
        &database,
        "shield.false_positive",
        "success",
        &format!("Message {id} reported as a false positive"),
    )?;
    load_analysis(&database, id)?
        .ok_or_else(|| "Could not read the stored security analysis".to_string())
}
