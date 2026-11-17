//! URL threat intelligence for Elyra Shield.
//!
//! Links extracted from messages are checked against the user's own
//! VirusTotal and Google Safe Browsing API keys. Results are cached in SQLite
//! so repeated opens do not burn provider quota, and every network failure is
//! reported as a per-provider status instead of blocking the mailbox.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration as StdDuration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use reqwest::blocking::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;

use crate::{integrations, storage};

const MAX_URLS: usize = 40;
const VIRUSTOTAL_MIN_INTERVAL_SECONDS: u64 = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderVerdict {
    pub provider: String,
    pub label: String,
    pub verdict: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlScan {
    pub url: String,
    pub host: String,
    pub verdict: String,
    pub score: i64,
    pub providers: Vec<ProviderVerdict>,
    pub cached: bool,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub urls: Vec<UrlScan>,
    pub scanned: bool,
    pub disabled: bool,
    pub providers: Vec<String>,
    pub note: String,
}

fn url_regex() -> &'static regex::Regex {
    static PATTERN: OnceLock<regex::Regex> = OnceLock::new();
    PATTERN.get_or_init(|| regex::Regex::new(r#"https?://[^\s<>"']+"#).expect("valid URL pattern"))
}

fn anchor_regex() -> &'static regex::Regex {
    static PATTERN: OnceLock<regex::Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        regex::Regex::new(r#"(?is)<a\s[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</a>"#)
            .expect("valid anchor pattern")
    })
}

fn virustotal_clock() -> &'static Mutex<Option<Instant>> {
    static CLOCK: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    CLOCK.get_or_init(|| Mutex::new(None))
}

pub(crate) fn allow_virustotal_request() -> bool {
    let clock = virustotal_clock();
    let mut guard = clock.lock().unwrap_or_else(|error| error.into_inner());
    let now = Instant::now();
    if let Some(last) = *guard {
        if now.duration_since(last) < StdDuration::from_secs(VIRUSTOTAL_MIN_INTERVAL_SECONDS) {
            return false;
        }
    }
    *guard = Some(now);
    true
}

pub(crate) fn host_of(url: &str) -> Option<String> {
    let without_scheme = url.split_once("://")?.1;
    let authority = without_scheme
        .split(|character| character == '/' || character == '?' || character == '#')
        .next()?;
    let host = authority.rsplit('@').next()?;
    let host = host.split(':').next()?.trim().trim_end_matches('.');
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

fn clean_url(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(|character: char| {
            matches!(character, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}')
        })
        .to_string()
}

pub(crate) fn extract_urls(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut urls = Vec::new();
    for found in url_regex().find_iter(text) {
        let url = clean_url(found.as_str());
        if url.len() < 10 {
            continue;
        }
        let key = url.to_ascii_lowercase();
        if seen.insert(key) {
            urls.push(url);
        }
        if urls.len() >= MAX_URLS {
            break;
        }
    }
    urls
}

pub(crate) fn link_pairs(html: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for captures in anchor_regex().captures_iter(html) {
        let href = captures.get(1).map(|value| value.as_str()).unwrap_or_default();
        if !href.starts_with("http://") && !href.starts_with("https://") {
            continue;
        }
        let text = captures.get(2).map(|value| value.as_str()).unwrap_or_default();
        let visible = url_regex()
            .find(text)
            .map(|found| clean_url(found.as_str()))
            .unwrap_or_default();
        pairs.push((clean_url(href), visible));
        if pairs.len() >= 20 {
            break;
        }
    }
    pairs
}

fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(StdDuration::from_secs(15))
        .user_agent(format!("Elyra/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Could not create the threat intelligence client: {error}"))
}

fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    database
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS threat_cache (
                url TEXT PRIMARY KEY,
                verdict TEXT NOT NULL,
                score INTEGER NOT NULL DEFAULT 0,
                providers TEXT NOT NULL DEFAULT '[]',
                checked_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_threat_cache_expires ON threat_cache(expires_at);",
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn external_scanning_enabled(database: &Connection) -> bool {
    database
        .query_row(
            "SELECT value FROM settings WHERE key = 'external_scanning'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
        .map(|value| !value.eq_ignore_ascii_case("false"))
        .unwrap_or(true)
}

fn cache_lookup(database: &Connection, url: &str) -> Result<Option<UrlScan>, String> {
    let row = database
        .query_row(
            "SELECT verdict, score, providers, checked_at FROM threat_cache
             WHERE url = ?1 AND expires_at > ?2",
            params![url, now_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((verdict, score, providers, checked_at)) = row else {
        return Ok(None);
    };
    Ok(Some(UrlScan {
        url: url.to_string(),
        host: host_of(url).unwrap_or_default(),
        verdict,
        score,
        providers: serde_json::from_str(&providers).unwrap_or_default(),
        cached: true,
        checked_at,
    }))
}

fn cache_store(database: &Connection, scan: &UrlScan) -> Result<(), String> {
    let hours = if scan.verdict == "unknown" { 1 } else { 24 };
    let expires_at = (Utc::now() + Duration::hours(hours))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let providers = serde_json::to_string(&scan.providers).unwrap_or_else(|_| "[]".to_string());
    database
        .execute(
            "INSERT INTO threat_cache (url, verdict, score, providers, checked_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(url) DO UPDATE SET
                verdict = excluded.verdict,
                score = excluded.score,
                providers = excluded.providers,
                checked_at = excluded.checked_at,
                expires_at = excluded.expires_at",
            params![
                scan.url,
                scan.verdict,
                scan.score,
                providers,
                scan.checked_at,
                expires_at
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn aggregate(verdicts: &[ProviderVerdict]) -> (String, i64) {
    let malicious = verdicts.iter().any(|item| item.verdict == "malicious");
    let suspicious = verdicts.iter().any(|item| item.verdict == "suspicious");
    let safe = verdicts.iter().any(|item| item.verdict == "safe");
    if malicious {
        ("malicious".to_string(), 70)
    } else if suspicious {
        ("suspicious".to_string(), 35)
    } else if safe {
        ("safe".to_string(), 0)
    } else {
        ("unknown".to_string(), 0)
    }
}

fn virustotal_lookup(client: &Client, key: &str, url: &str) -> Result<ProviderVerdict, String> {
    let identifier = URL_SAFE_NO_PAD.encode(url.as_bytes());
    let response = client
        .get(format!(
            "https://www.virustotal.com/api/v3/urls/{identifier}"
        ))
        .header("x-apikey", key)
        .send()
        .map_err(|error| format!("VirusTotal request failed: {error}"))?;
    let status = response.status();
    if status.as_u16() == 404 {
        return Ok(ProviderVerdict {
            provider: integrations::VIRUSTOTAL.to_string(),
            label: integrations::provider_label(integrations::VIRUSTOTAL).to_string(),
            verdict: "unknown".to_string(),
            detail: "VirusTotal has not analyzed this link yet".to_string(),
        });
    }
    if status.as_u16() == 429 {
        return Ok(ProviderVerdict {
            provider: integrations::VIRUSTOTAL.to_string(),
            label: integrations::provider_label(integrations::VIRUSTOTAL).to_string(),
            verdict: "skipped".to_string(),
            detail: "VirusTotal rate limit reached; try again later".to_string(),
        });
    }
    if !status.is_success() {
        return Err(format!("VirusTotal returned HTTP {status}"));
    }
    let payload: serde_json::Value = response
        .json()
        .map_err(|error| format!("VirusTotal response was not valid JSON: {error}"))?;
    let stats = payload
        .pointer("/data/attributes/last_analysis_stats")
        .ok_or_else(|| "VirusTotal response did not include analysis statistics".to_string())?;
    let malicious = stats.get("malicious").and_then(|value| value.as_i64()).unwrap_or(0);
    let suspicious = stats.get("suspicious").and_then(|value| value.as_i64()).unwrap_or(0);
    let harmless = stats.get("harmless").and_then(|value| value.as_i64()).unwrap_or(0);
    let verdict = if malicious > 0 {
        "malicious"
    } else if suspicious > 0 {
        "suspicious"
    } else {
        "safe"
    };
    Ok(ProviderVerdict {
        provider: integrations::VIRUSTOTAL.to_string(),
        label: integrations::provider_label(integrations::VIRUSTOTAL).to_string(),
        verdict: verdict.to_string(),
        detail: format!("{malicious} malicious, {suspicious} suspicious, {harmless} clean vendors"),
    })
}

fn google_lookup(
    client: &Client,
    key: &str,
    urls: &[String],
) -> Result<HashMap<String, ProviderVerdict>, String> {
    let entries: Vec<serde_json::Value> = urls.iter().map(|url| json!({ "url": url })).collect();
    let body = json!({
        "client": {
            "clientId": "com.elyra.desktop",
            "clientVersion": env!("CARGO_PKG_VERSION")
        },
        "threatInfo": {
            "threatTypes": [
                "MALWARE",
                "SOCIAL_ENGINEERING",
                "UNWANTED_SOFTWARE",
                "POTENTIALLY_HARMFUL_APPLICATION"
            ],
            "platformTypes": ["ANY_PLATFORM"],
            "threatEntryTypes": ["URL"],
            "threatEntries": entries
        }
    });
    let response = client
        .post("https://safebrowsing.googleapis.com/v4/threatMatches:find")
        .query(&[("key", key)])
        .json(&body)
        .send()
        .map_err(|error| format!("Google Safe Browsing request failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        let detail = response.text().unwrap_or_default();
        let short = detail.chars().take(160).collect::<String>();
        return Err(format!("Google Safe Browsing returned HTTP {status}: {short}"));
    }
    let payload: serde_json::Value = response
        .json()
        .map_err(|error| format!("Google Safe Browsing response was not valid JSON: {error}"))?;
    let mut matches: HashMap<String, String> = HashMap::new();
    if let Some(items) = payload.get("matches").and_then(|value| value.as_array()) {
        for item in items {
            let threat_type = item
                .get("threatType")
                .and_then(|value| value.as_str())
                .unwrap_or("MALWARE");
            if let Some(flagged) = item
                .pointer("/threat/url")
                .and_then(|value| value.as_str())
            {
                if let Some(host) = host_of(flagged) {
                    matches.insert(host, threat_type.to_string());
                }
            }
        }
    }
    let mut results = HashMap::new();
    for url in urls {
        let host = host_of(url).unwrap_or_default();
        let verdict = match matches.get(&host) {
            Some(threat_type) => ProviderVerdict {
                provider: integrations::GOOGLE_SAFE_BROWSING.to_string(),
                label: integrations::provider_label(integrations::GOOGLE_SAFE_BROWSING)
                    .to_string(),
                verdict: "malicious".to_string(),
                detail: format!("Google Safe Browsing matched {threat_type}"),
            },
            None => ProviderVerdict {
                provider: integrations::GOOGLE_SAFE_BROWSING.to_string(),
                label: integrations::provider_label(integrations::GOOGLE_SAFE_BROWSING)
                    .to_string(),
                verdict: "safe".to_string(),
                detail: "No Google Safe Browsing matches".to_string(),
            },
        };
        results.insert(url.clone(), verdict);
    }
    Ok(results)
}

fn skipped_verdict(provider: &str, detail: &str) -> ProviderVerdict {
    ProviderVerdict {
        provider: provider.to_string(),
        label: integrations::provider_label(provider).to_string(),
        verdict: "skipped".to_string(),
        detail: detail.to_string(),
    }
}

pub(crate) fn scan_urls(app: &AppHandle, urls: Vec<String>) -> Result<ScanReport, String> {
    let database = storage::connection(app)?;
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for url in urls {
        let cleaned = clean_url(&url);
        if cleaned.len() < 10 || !seen.insert(cleaned.to_ascii_lowercase()) {
            continue;
        }
        unique.push(cleaned);
        if unique.len() >= MAX_URLS {
            break;
        }
    }

    if unique.is_empty() {
        return Ok(ScanReport {
            urls: Vec::new(),
            scanned: false,
            disabled: !external_scanning_enabled(&database),
            providers: Vec::new(),
            note: "No links found in this message.".to_string(),
        });
    }

    let enabled = external_scanning_enabled(&database);
    let active = integrations::active_providers(app)?;
    let provider_labels: Vec<String> = active
        .iter()
        .map(|(provider, _)| integrations::provider_label(provider).to_string())
        .collect();
    let provider_keys: HashMap<String, String> = active.into_iter().collect();

    let mut results = Vec::new();
    let mut pending = Vec::new();
    for url in &unique {
        if let Some(cached) = cache_lookup(&database, url)? {
            results.push(cached);
        } else {
            pending.push(url.clone());
        }
    }

    if pending.is_empty() {
        return Ok(ScanReport {
            urls: results,
            scanned: true,
            disabled: !enabled,
            providers: provider_labels,
            note: "Every link was served from the local threat cache.".to_string(),
        });
    }

    if !enabled || provider_keys.is_empty() {
        let reason = if !enabled {
            "External link scanning is turned off in Settings."
        } else {
            "Connect VirusTotal or Google Safe Browsing in Settings to check links."
        };
        for url in pending {
            results.push(UrlScan {
                host: host_of(&url).unwrap_or_default(),
                url,
                verdict: "unscanned".to_string(),
                score: 0,
                providers: Vec::new(),
                cached: false,
                checked_at: now_string(),
            });
        }
        return Ok(ScanReport {
            urls: results,
            scanned: false,
            disabled: !enabled,
            providers: provider_labels,
            note: reason.to_string(),
        });
    }

    let threat_client = client()?;
    let mut google_verdicts: HashMap<String, ProviderVerdict> = HashMap::new();
    let mut google_error: Option<String> = None;
    if let Some(key) = provider_keys.get(integrations::GOOGLE_SAFE_BROWSING) {
        match google_lookup(&threat_client, key, &pending) {
            Ok(verdicts) => google_verdicts = verdicts,
            Err(error) => google_error = Some(error),
        }
    }

    for url in pending {
        let mut verdicts = Vec::new();
        if provider_keys.contains_key(integrations::GOOGLE_SAFE_BROWSING) {
            if let Some(verdict) = google_verdicts.get(&url) {
                verdicts.push(verdict.clone());
            } else if let Some(error) = &google_error {
                verdicts.push(skipped_verdict(
                    integrations::GOOGLE_SAFE_BROWSING,
                    &format!("Safe Browsing check unavailable: {error}"),
                ));
            }
        }
        if let Some(key) = provider_keys.get(integrations::VIRUSTOTAL) {
            if allow_virustotal_request() {
                match virustotal_lookup(&threat_client, key, &url) {
                    Ok(verdict) => verdicts.push(verdict),
                    Err(error) => verdicts.push(skipped_verdict(
                        integrations::VIRUSTOTAL,
                        &format!("VirusTotal check unavailable: {error}"),
                    )),
                }
            } else {
                verdicts.push(skipped_verdict(
                    integrations::VIRUSTOTAL,
                    "VirusTotal free-tier limit: one request every 16 seconds",
                ));
            }
        }
        let (verdict, score) = aggregate(&verdicts);
        let scan = UrlScan {
            host: host_of(&url).unwrap_or_default(),
            url,
            verdict,
            score,
            providers: verdicts,
            cached: false,
            checked_at: now_string(),
        };
        cache_store(&database, &scan)?;
        results.push(scan);
    }

    Ok(ScanReport {
        urls: results,
        scanned: true,
        disabled: false,
        providers: provider_labels,
        note: "Link checks completed with your connected providers.".to_string(),
    })
}

pub(crate) fn validate_provider(provider: &str, key: &str) -> Result<String, String> {
    let threat_client = client()?;
    let key = key.trim();
    if key.len() < 8 {
        return Err("The API key looks too short.".to_string());
    }
    match provider {
        integrations::VIRUSTOTAL => {
            let response = threat_client
                .get("https://www.virustotal.com/api/v3/users/current")
                .header("x-apikey", key)
                .send()
                .map_err(|error| format!("Could not reach VirusTotal: {error}"))?;
            match response.status().as_u16() {
                200 => Ok("VirusTotal accepted the API key.".to_string()),
                401 | 403 => Err("VirusTotal rejected this API key.".to_string()),
                status => Err(format!("VirusTotal returned HTTP {status} while validating the key.")),
            }
        }
        integrations::GOOGLE_SAFE_BROWSING => {
            let body = json!({
                "client": {
                    "clientId": "com.elyra.desktop",
                    "clientVersion": env!("CARGO_PKG_VERSION")
                },
                "threatInfo": {
                    "threatTypes": ["MALWARE"],
                    "platformTypes": ["ANY_PLATFORM"],
                    "threatEntryTypes": ["URL"],
                    "threatEntries": [{ "url": "https://example.com/" }]
                }
            });
            let response = threat_client
                .post("https://safebrowsing.googleapis.com/v4/threatMatches:find")
                .query(&[("key", key)])
                .json(&body)
                .send()
                .map_err(|error| format!("Could not reach Google Safe Browsing: {error}"))?;
            let status = response.status();
            if status.is_success() {
                Ok("Google Safe Browsing accepted the API key.".to_string())
            } else if status.as_u16() == 400 || status.as_u16() == 403 {
                Err("Google Safe Browsing rejected this API key or the API is not enabled.".to_string())
            } else {
                Err(format!(
                    "Google Safe Browsing returned HTTP {status} while validating the key."
                ))
            }
        }
        _ => Err("Unknown threat intelligence provider.".to_string()),
    }
}

#[tauri::command(async)]
pub fn scan_links(app: AppHandle, text: String) -> Result<ScanReport, String> {
    scan_urls(&app, extract_urls(&text))
}
