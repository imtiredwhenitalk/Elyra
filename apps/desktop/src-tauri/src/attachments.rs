//! Attachment security scanning.
//!
//! Attachments are classified from their real bytes, not only from the name or
//! the MIME type declared by the sender. Local heuristics always run on this
//! device. ClamAV is optional and only used when the user turns it on. VirusTotal
//! file analysis is never automatic: every upload needs explicit consent and a
//! stored API key. Files are never executed or opened by Elyra.

use std::process::{Command, Stdio};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use reqwest::blocking::{multipart, Client};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::AppHandle;

use crate::{integrations, mailbox, secrets, security, storage, threats};

const DEFAULT_MAX_MB: i64 = 25;
const MAX_DOWNLOAD_BYTES: i64 = 50 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentScan {
    pub attachment_id: i64,
    pub email_id: i64,
    pub name: String,
    pub mime_type: String,
    pub detected_mime: String,
    pub kind: String,
    pub size_bytes: i64,
    pub sha256: Option<String>,
    pub risk: String,
    pub score: i64,
    pub reasons: Vec<String>,
    pub content_available: bool,
    pub scan_source: String,
    pub scanned_at: Option<String>,
    pub consent_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentContent {
    pub id: i64,
    pub name: String,
    pub mime_type: String,
    pub detected_mime: String,
    pub size_bytes: i64,
    pub data_base64: String,
}

struct StoredAttachment {
    id: i64,
    email_id: i64,
    name: String,
    mime_type: String,
    size_bytes: i64,
    content: Option<Vec<u8>>,
    sha256: Option<String>,
    detected_mime: String,
    kind: String,
    risk: String,
    reasons: String,
    content_available: bool,
    score: i64,
    scan_source: String,
    scanned_at: Option<String>,
    consent_required: bool,
}

struct Assessment {
    detected_mime: String,
    kind: String,
    risk: String,
    score: i64,
    reasons: Vec<String>,
}

enum ClamOutcome {
    Clean,
    Infected(String),
}

fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn setting_value(database: &Connection, key: &str) -> Option<String> {
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

fn setting_enabled(database: &Connection, key: &str, fallback: bool) -> bool {
    match setting_value(database, key) {
        Some(value) => !value.eq_ignore_ascii_case("false"),
        None => fallback,
    }
}

fn max_scan_bytes(database: &Connection) -> i64 {
    setting_value(database, "attachment_scan_max_mb")
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(DEFAULT_MAX_MB)
        .clamp(1, 200)
        * 1024
        * 1024
}

fn clamav_path(database: &Connection) -> String {
    setting_value(database, "clamav_path")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "clamscan".to_string())
}

const ATTACHMENT_COLUMNS: &[(&str, &str)] = &[
    ("content", "BLOB"),
    ("sha256", "TEXT"),
    ("detected_mime", "TEXT NOT NULL DEFAULT ''"),
    ("kind", "TEXT NOT NULL DEFAULT 'other'"),
    ("risk", "TEXT NOT NULL DEFAULT 'unknown'"),
    ("reasons", "TEXT NOT NULL DEFAULT '[]'"),
    ("content_available", "INTEGER NOT NULL DEFAULT 0"),
    ("score", "INTEGER NOT NULL DEFAULT 0"),
    ("scan_source", "TEXT NOT NULL DEFAULT 'local'"),
    ("scanned_at", "TEXT"),
    ("consent_required", "INTEGER NOT NULL DEFAULT 0"),
];

/// Add the attachment-scanning columns. Safe to call more than once and also
/// used by the mailbox module so demo seeding can store attachment content.
pub(crate) fn ensure_columns(database: &Connection) -> Result<(), String> {
    for &(column, definition) in ATTACHMENT_COLUMNS {
        mailbox::ensure_column(database, "attachments", column, definition)?;
    }
    Ok(())
}

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    ensure_columns(&database)?;
    for (key, value) in [
        ("attachment_scanning", "true"),
        ("attachment_clamav", "false"),
        ("attachment_virustotal", "false"),
        ("clamav_path", "clamscan"),
        ("attachment_scan_max_mb", "25"),
    ] {
        database
            .execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|error| error.to_string())?;
    }
    // Classify attachments that were imported before this module existed.
    let _ = scan_pending(&database);
    Ok(())
}

fn extension_of(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, extension)| extension.trim().to_ascii_lowercase())
        .unwrap_or_default()
}

fn is_executable_extension(extension: &str) -> bool {
    matches!(
        extension,
        "exe"
            | "scr"
            | "com"
            | "pif"
            | "msi"
            | "msp"
            | "lnk"
            | "dll"
            | "sys"
            | "cpl"
            | "apk"
            | "app"
            | "deb"
            | "rpm"
            | "dmg"
            | "pkg"
            | "iso"
            | "img"
            | "vhd"
            | "vhdx"
            | "vbs"
            | "vbe"
            | "js"
            | "jse"
            | "wsf"
            | "wsh"
            | "hta"
            | "bat"
            | "cmd"
            | "ps1"
            | "sh"
            | "jar"
    )
}

fn is_script_extension(extension: &str) -> bool {
    matches!(
        extension,
        "js" | "jse" | "vbs" | "vbe" | "wsf" | "wsh" | "hta" | "bat" | "cmd" | "ps1" | "sh" | "py" | "rb" | "pl"
    )
}

fn is_disk_image_extension(extension: &str) -> bool {
    matches!(extension, "iso" | "img" | "vhd" | "vhdx")
}

fn is_macro_extension(extension: &str) -> bool {
    matches!(
        extension,
        "docm" | "dotm" | "xlsm" | "xltm" | "xlam" | "pptm" | "potm" | "ppam"
    )
}

fn is_archive_extension(extension: &str) -> bool {
    matches!(
        extension,
        "zip" | "rar" | "7z" | "gz" | "bz2" | "xz" | "tar" | "cab" | "arj" | "lzh" | "z" | "tgz" | "tbz2"
    )
}

fn is_document_extension(extension: &str) -> bool {
    matches!(
        extension,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "rtf" | "txt" | "csv" | "md"
    )
}

fn is_image_extension(extension: &str) -> bool {
    matches!(
        extension,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "svg" | "heic" | "avif"
    )
}

fn is_disguised_extension(extension: &str) -> bool {
    matches!(
        extension,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "csv" | "jpg" | "jpeg" | "png" | "gif" | "zip"
    )
}

fn starts_with(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix
}

fn contains_bytes(bytes: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && bytes.windows(needle.len()).any(|window| window == needle)
}

fn looks_like_text(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let sample = &bytes[..bytes.len().min(4096)];
    sample.iter().all(|byte| {
        matches!(*byte, 9 | 10 | 13) || (32..=126).contains(byte) || *byte >= 0x80
    }) && sample
        .iter()
        .filter(|byte| **byte < 0x09)
        .count()
        == 0
}

fn sniff_mime(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    if starts_with(bytes, b"%PDF-") {
        return Some("application/pdf".to_string());
    }
    if starts_with(bytes, b"\x89PNG\r\n\x1a\n") {
        return Some("image/png".to_string());
    }
    if starts_with(bytes, b"\xff\xd8\xff") {
        return Some("image/jpeg".to_string());
    }
    if starts_with(bytes, b"GIF87a") || starts_with(bytes, b"GIF89a") {
        return Some("image/gif".to_string());
    }
    if starts_with(bytes, b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
        return Some("image/webp".to_string());
    }
    if starts_with(bytes, b"BM") {
        return Some("image/bmp".to_string());
    }
    if starts_with(bytes, b"II*\0") || starts_with(bytes, b"MM\0*") {
        return Some("image/tiff".to_string());
    }
    if starts_with(bytes, b"PK\x03\x04")
        || starts_with(bytes, b"PK\x05\x06")
        || starts_with(bytes, b"PK\x07\x08")
    {
        if contains_bytes(bytes, b"word/") {
            return Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                    .to_string(),
            );
        }
        if contains_bytes(bytes, b"xl/") {
            return Some(
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            );
        }
        if contains_bytes(bytes, b"ppt/") {
            return Some(
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                    .to_string(),
            );
        }
        return Some("application/zip".to_string());
    }
    if starts_with(bytes, b"Rar!\x1a\x07") {
        return Some("application/vnd.rar".to_string());
    }
    if starts_with(bytes, b"7z\xbc\xaf\x27\x1c") {
        return Some("application/x-7z-compressed".to_string());
    }
    if starts_with(bytes, b"\x1f\x8b") {
        return Some("application/gzip".to_string());
    }
    if starts_with(bytes, b"BZh") {
        return Some("application/x-bzip2".to_string());
    }
    if starts_with(bytes, b"\xfd7zXZ\x00") {
        return Some("application/x-xz".to_string());
    }
    if bytes.len() > 262 && &bytes[257..262] == b"ustar" {
        return Some("application/x-tar".to_string());
    }
    if starts_with(bytes, b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1") {
        return Some("application/x-ole-storage".to_string());
    }
    if starts_with(bytes, b"MZ") {
        return Some("application/x-dosexec".to_string());
    }
    if starts_with(bytes, b"\x7fELF") {
        return Some("application/x-elf".to_string());
    }
    if starts_with(bytes, b"\xca\xfe\xba\xbe")
        || starts_with(bytes, b"\xfe\xed\xfa\xce")
        || starts_with(bytes, b"\xfe\xed\xfa\xcf")
        || starts_with(bytes, b"\xce\xfa\xed\xfe")
        || starts_with(bytes, b"\xcf\xfa\xed\xfe")
    {
        return Some("application/x-mach-binary".to_string());
    }
    if starts_with(bytes, b"{\\rtf") {
        return Some("application/rtf".to_string());
    }
    if starts_with(bytes, b"#!") {
        return Some("text/x-script".to_string());
    }

    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(2048)])
        .trim_start()
        .to_ascii_lowercase();
    if text.starts_with("<!doctype html") || text.starts_with("<html") {
        return Some("text/html".to_string());
    }
    if text.starts_with("<svg") || text.contains("<svg") {
        return Some("image/svg+xml".to_string());
    }
    if looks_like_text(bytes) {
        return Some("text/plain".to_string());
    }
    None
}

fn mime_family(mime: &str) -> &'static str {
    let mime = mime.to_ascii_lowercase();
    if mime.starts_with("image/") {
        "image"
    } else if mime.starts_with("text/") {
        "text"
    } else if mime == "application/pdf" {
        "pdf"
    } else if mime.contains("zip")
        || mime.contains("rar")
        || mime.contains("7z")
        || mime.contains("gzip")
        || mime.contains("x-tar")
        || mime.contains("compressed")
        || mime.contains("bzip2")
        || mime.contains("x-xz")
    {
        "archive"
    } else if mime.contains("officedocument")
        || mime.contains("msword")
        || mime.contains("ms-excel")
        || mime.contains("ms-powerpoint")
        || mime.contains("ole-storage")
        || mime == "application/rtf"
    {
        "document"
    } else if mime.contains("dosexec")
        || mime.contains("elf")
        || mime.contains("mach-binary")
        || mime.contains("x-msdownload")
    {
        "executable"
    } else {
        "other"
    }
}

fn kind_for(detected_mime: &str, extension: &str) -> &'static str {
    let family = mime_family(detected_mime);
    if is_disk_image_extension(extension) {
        "disk-image"
    } else if family == "executable" || is_executable_extension(extension) {
        if is_script_extension(extension) || detected_mime == "text/x-script" {
            "script"
        } else {
            "executable"
        }
    } else if is_macro_extension(extension) {
        "macro"
    } else if family == "archive" || is_archive_extension(extension) {
        "archive"
    } else if family == "image" || is_image_extension(extension) {
        "image"
    } else if family == "document" || is_document_extension(extension) {
        "document"
    } else {
        "other"
    }
}

fn risk_for(score: i64) -> &'static str {
    if score >= 60 {
        "malicious"
    } else if score >= 25 {
        "suspicious"
    } else {
        "safe"
    }
}

fn assess(name: &str, declared_mime: &str, content: Option<&[u8]>) -> Assessment {
    let extension = extension_of(name);
    let sniffed = content.and_then(sniff_mime);
    let declared = declared_mime.trim().to_ascii_lowercase();
    let detected_mime = sniffed
        .clone()
        .or_else(|| {
            if declared.is_empty() {
                None
            } else {
                Some(declared.clone())
            }
        })
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let kind = kind_for(&detected_mime, &extension).to_string();
    let mut score = 0_i64;
    let mut reasons: Vec<String> = Vec::new();

    if is_disk_image_extension(&extension) {
        score += 30;
        reasons.push(format!(
            "The attachment is a disk image ({extension}) and can carry hidden executables."
        ));
    } else if is_executable_extension(&extension) && is_script_extension(&extension) {
        score += 35;
        reasons.push(format!(
            "The attachment is a script ({extension}) and could run code when opened."
        ));
    } else if is_executable_extension(&extension) {
        score += 45;
        reasons.push(format!(
            "The attachment is an executable program ({extension}) and could harm this device."
        ));
    }

    if let Some(sniffed_mime) = sniffed.as_deref() {
        let detected_family = mime_family(sniffed_mime);
        if detected_family == "executable" && !is_executable_extension(&extension) {
            score += 50;
            reasons.push(format!(
                "The file content is executable ({sniffed_mime}), but the name does not show an executable extension."
            ));
        } else if !declared.is_empty()
            && !declared.eq_ignore_ascii_case("application/octet-stream")
            && mime_family(&declared) != detected_family
            && !(mime_family(&declared) == "document" && detected_family == "archive")
            && !(mime_family(&declared) == "text" && detected_family == "text")
        {
            score += 30;
            reasons.push(format!(
                "The file content looks like {sniffed_mime}, but the message declared {declared}."
            ));
        }
    }

    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() >= 3 {
        let inner = parts[parts.len() - 2].trim().to_ascii_lowercase();
        if is_disguised_extension(&inner) && is_executable_extension(&extension) {
            score += 35;
            reasons.push(
                "The name uses a double extension to disguise an executable as a document."
                    .to_string(),
            );
        }
    }

    if name.chars().any(|character| {
        matches!(
            character,
            '\u{202A}'
                | '\u{202B}'
                | '\u{202C}'
                | '\u{202D}'
                | '\u{202E}'
                | '\u{2066}'
                | '\u{2067}'
                | '\u{2068}'
                | '\u{2069}'
        )
    }) {
        score += 40;
        reasons.push(
            "The file name contains bidirectional text controls that can hide the real extension."
                .to_string(),
        );
    }

    if is_macro_extension(&extension) {
        score += 25;
        reasons.push(format!(
            "The attachment is a macro-enabled Office document ({extension})."
        ));
    }

    if kind == "archive" && !is_disk_image_extension(&extension) {
        score += 25;
        reasons.push(
            "The attachment is an archive. Archives can hide executables, so scan it before opening."
                .to_string(),
        );
    }

    if detected_mime == "text/html" {
        score += 25;
        reasons.push(
            "The attachment is a web page. HTML attachments are often used for phishing."
                .to_string(),
        );
    } else if detected_mime == "image/svg+xml" {
        score += 10;
        reasons.push(
            "The attachment is an SVG image, which can contain active content.".to_string(),
        );
    }

    let score = score.min(100);
    Assessment {
        detected_mime,
        kind,
        risk: risk_for(score).to_string(),
        score,
        reasons,
    }
}

fn load_attachment(database: &Connection, id: i64) -> Result<StoredAttachment, String> {
    database
        .query_row(
            "SELECT id, email_id, name, mime_type, size_bytes, content, sha256,
                    detected_mime, kind, risk, reasons, content_available,
                    score, scan_source, scanned_at, consent_required
             FROM attachments WHERE id = ?1",
            params![id],
            |row| {
                Ok(StoredAttachment {
                    id: row.get(0)?,
                    email_id: row.get(1)?,
                    name: row.get(2)?,
                    mime_type: row.get(3)?,
                    size_bytes: row.get(4)?,
                    content: row.get(5)?,
                    sha256: row.get(6)?,
                    detected_mime: row.get(7)?,
                    kind: row.get(8)?,
                    risk: row.get(9)?,
                    reasons: row.get(10)?,
                    content_available: row.get::<_, i64>(11)? != 0,
                    score: row.get(12)?,
                    scan_source: row.get(13)?,
                    scanned_at: row.get(14)?,
                    consent_required: row.get::<_, i64>(15)? != 0,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Attachment not found".to_string())
}

fn view_of(stored: &StoredAttachment) -> AttachmentScan {
    AttachmentScan {
        attachment_id: stored.id,
        email_id: stored.email_id,
        name: stored.name.clone(),
        mime_type: stored.mime_type.clone(),
        detected_mime: stored.detected_mime.clone(),
        kind: stored.kind.clone(),
        size_bytes: stored.size_bytes,
        sha256: stored.sha256.clone(),
        risk: stored.risk.clone(),
        score: stored.score,
        reasons: serde_json::from_str(&stored.reasons).unwrap_or_default(),
        content_available: stored.content_available,
        scan_source: stored.scan_source.clone(),
        scanned_at: stored.scanned_at.clone(),
        consent_required: stored.consent_required,
    }
}

fn store_scan(
    database: &Connection,
    id: i64,
    detected_mime: &str,
    kind: &str,
    risk: &str,
    score: i64,
    reasons: &[String],
    sha256: Option<&str>,
    content_available: bool,
    scan_source: &str,
) -> Result<(), String> {
    let reasons_json = serde_json::to_string(reasons).unwrap_or_else(|_| "[]".to_string());
    // Stored content makes an external upload possible, so any upload needs
    // explicit per-file consent. Metadata-only attachments cannot be uploaded.
    let consent_required = if content_available { 1 } else { 0 };
    database
        .execute(
            "UPDATE attachments
             SET detected_mime = ?2, kind = ?3, risk = ?4, reasons = ?5, sha256 = ?6,
                 content_available = ?7, score = ?8, scan_source = ?9, scanned_at = ?10,
                 consent_required = ?11
             WHERE id = ?1",
            params![
                id,
                detected_mime,
                kind,
                risk,
                reasons_json,
                sha256,
                if content_available { 1 } else { 0 },
                score,
                scan_source,
                now_string(),
                consent_required
            ],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn sha256_of(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn clamav_scan(database: &Connection, id: i64, bytes: &[u8]) -> Result<ClamOutcome, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("elyra-scan-{id}-{stamp}.bin"));
    std::fs::write(&path, bytes)
        .map_err(|error| format!("could not write a temporary file ({error})"))?;
    let output = Command::new(clamav_path(database))
        .arg("--no-summary")
        .arg("--stdout")
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let _ = std::fs::remove_file(&path);
    let output = output.map_err(|error| format!("could not start ClamAV ({error})"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    match output.status.code() {
        Some(0) => Ok(ClamOutcome::Clean),
        Some(1) => {
            let detail = stdout
                .lines()
                .find(|line| line.contains("FOUND"))
                .unwrap_or("ClamAV detected malware")
                .trim()
                .chars()
                .take(200)
                .collect::<String>();
            Ok(ClamOutcome::Infected(detail))
        }
        Some(code) => Err(format!(
            "ClamAV exited with code {code}: {}",
            stderr.trim().chars().take(200).collect::<String>()
        )),
        None => Err("ClamAV was stopped before it finished".to_string()),
    }
}

fn scan_one(
    database: &Connection,
    id: i64,
    run_clamav: bool,
) -> Result<AttachmentScan, String> {
    let stored = load_attachment(database, id)?;
    let assessment = assess(
        &stored.name,
        &stored.mime_type,
        stored.content.as_deref(),
    );
    let sha256 = stored.content.as_deref().map(sha256_of);
    let mut risk = assessment.risk.clone();
    let mut score = assessment.score;
    let mut reasons = assessment.reasons.clone();
    let mut scan_source = "local".to_string();

    if run_clamav && setting_enabled(database, "attachment_clamav", false) {
        match stored.content.as_deref() {
            Some(bytes) if !bytes.is_empty() => {
                if stored.size_bytes > max_scan_bytes(database) {
                    reasons.push(format!(
                        "The file is larger than the ClamAV scan limit of {} MB.",
                        max_scan_bytes(database) / (1024 * 1024)
                    ));
                } else {
                    match clamav_scan(database, stored.id, bytes) {
                        Ok(ClamOutcome::Clean) => {
                            scan_source = "clamav".to_string();
                            reasons.push("ClamAV found no malware in this file.".to_string());
                        }
                        Ok(ClamOutcome::Infected(detail)) => {
                            scan_source = "clamav".to_string();
                            risk = "malicious".to_string();
                            score = 100;
                            reasons.push(format!("ClamAV detected malware: {detail}"));
                        }
                        Err(error) => {
                            reasons.push(format!("ClamAV could not scan this file: {error}"));
                        }
                    }
                }
            }
            Some(_) => {
                reasons.push("The attachment is empty, so there was nothing to scan.".to_string());
            }
            None => {
                reasons.push(
                    "The attachment content is not stored locally, so ClamAV could not scan it."
                        .to_string(),
                );
            }
        }
    }

    store_scan(
        database,
        stored.id,
        &assessment.detected_mime,
        &assessment.kind,
        &risk,
        score,
        &reasons,
        sha256.as_deref(),
        stored.content_available,
        &scan_source,
    )?;
    Ok(view_of(&load_attachment(database, stored.id)?))
}

fn attachment_ids(database: &Connection, email_id: Option<i64>) -> Result<Vec<i64>, String> {
    let mut ids = Vec::new();
    match email_id {
        Some(email_id) => {
            let mut statement = database
                .prepare("SELECT id FROM attachments WHERE email_id = ?1 ORDER BY id")
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map(params![email_id], |row| row.get::<_, i64>(0))
                .map_err(|error| error.to_string())?;
            for row in rows {
                ids.push(row.map_err(|error| error.to_string())?);
            }
        }
        None => {
            let mut statement = database
                .prepare("SELECT id FROM attachments ORDER BY id")
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| row.get::<_, i64>(0))
                .map_err(|error| error.to_string())?;
            for row in rows {
                ids.push(row.map_err(|error| error.to_string())?);
            }
        }
    }
    Ok(ids)
}

pub(crate) fn scan_email_attachments(
    database: &Connection,
    email_id: i64,
) -> Result<i64, String> {
    if !setting_enabled(database, "attachment_scanning", true) {
        return Ok(0);
    }
    let ids = attachment_ids(database, Some(email_id))?;
    let mut scanned = 0;
    for id in ids {
        if scan_one(database, id, true).is_ok() {
            scanned += 1;
        }
    }
    Ok(scanned)
}

fn scan_pending(database: &Connection) -> Result<i64, String> {
    let mut statement = database
        .prepare("SELECT id FROM attachments WHERE detected_mime = '' OR scanned_at IS NULL")
        .map_err(|error| error.to_string())?;
    let ids = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    let mut scanned = 0;
    for id in ids {
        if scan_one(database, id, false).is_ok() {
            scanned += 1;
        }
    }
    Ok(scanned)
}

#[tauri::command(async)]
pub fn scan_attachment(app: AppHandle, id: i64) -> Result<AttachmentScan, String> {
    let database = storage::connection(&app)?;
    scan_one(&database, id, true)
}

#[tauri::command(async)]
pub fn scan_all_attachments(app: AppHandle) -> Result<i64, String> {
    let database = storage::connection(&app)?;
    let ids = attachment_ids(&database, None)?;
    let mut scanned = 0;
    for id in ids {
        if scan_one(&database, id, true).is_ok() {
            scanned += 1;
        }
    }
    Ok(scanned)
}

#[tauri::command(async)]
pub fn scan_attachment_external(
    app: AppHandle,
    id: i64,
    consent: bool,
) -> Result<AttachmentScan, String> {
    if !consent {
        return Err(
            "Explicit consent is required before an attachment can be uploaded for scanning."
                .to_string(),
        );
    }
    let database = storage::connection(&app)?;
    if !setting_enabled(&database, "attachment_virustotal", false) {
        return Err(
            "Turn on VirusTotal attachment analysis in Settings before uploading a file."
                .to_string(),
        );
    }
    let stored = load_attachment(&database, id)?;
    let bytes = stored
        .content
        .as_deref()
        .filter(|bytes| !bytes.is_empty())
        .ok_or_else(|| "This attachment has no stored content to upload.".to_string())?;
    if stored.size_bytes > max_scan_bytes(&database) {
        return Err(format!(
            "The file is larger than the {} MB attachment scan limit.",
            max_scan_bytes(&database) / (1024 * 1024)
        ));
    }
    let key = secrets::get_secret(&secrets::integration_user(integrations::VIRUSTOTAL))?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Connect VirusTotal in Settings first.".to_string())?;
    if !threats::allow_virustotal_request() {
        return Err(
            "VirusTotal free-tier limit: one request every 16 seconds. Try again shortly."
                .to_string(),
        );
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(90))
        .user_agent(format!("Elyra/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Could not create the VirusTotal client: {error}"))?;
    let form = multipart::Form::new().part(
        "file",
        multipart::Part::bytes(bytes.to_vec()).file_name(stored.name.clone()),
    );
    let response = client
        .post("https://www.virustotal.com/api/v3/files")
        .header("x-apikey", key.trim())
        .multipart(form)
        .send()
        .map_err(|error| format!("VirusTotal upload failed: {error}"))?;
    let status = response.status();
    if status.as_u16() == 429 {
        return Err("VirusTotal rate limit reached. Try again later.".to_string());
    }
    if !status.is_success() {
        let detail = response.text().unwrap_or_default();
        let short = detail.chars().take(180).collect::<String>();
        let _ = security::record_event(
            &database,
            "attachment.external_scan",
            "failure",
            &format!("VirusTotal rejected attachment #{id}: HTTP {status}"),
        );
        return Err(format!(
            "VirusTotal returned HTTP {status}: {short}"
        ));
    }
    let payload: serde_json::Value = response
        .json()
        .map_err(|error| format!("VirusTotal response was not valid JSON: {error}"))?;
    let analysis_id = payload
        .pointer("/data/id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "VirusTotal did not return an analysis id.".to_string())?;
    let _ = security::record_event(
        &database,
        "attachment.external_scan",
        "success",
        &format!("Uploaded attachment #{id} to VirusTotal with explicit consent"),
    );

    let mut stats: Option<serde_json::Value> = None;
    for attempt in 0..4 {
        let response = client
            .get(format!(
                "https://www.virustotal.com/api/v3/analyses/{analysis_id}"
            ))
            .header("x-apikey", key.trim())
            .send()
            .map_err(|error| format!("VirusTotal analysis request failed: {error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "VirusTotal returned HTTP {} while reading the analysis.",
                response.status()
            ));
        }
        let payload: serde_json::Value = response
            .json()
            .map_err(|error| format!("VirusTotal response was not valid JSON: {error}"))?;
        let analysis_status = payload
            .pointer("/data/attributes/status")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if analysis_status == "completed" {
            stats = payload
                .pointer("/data/attributes/stats")
                .cloned();
            break;
        }
        if attempt < 3 {
            std::thread::sleep(Duration::from_secs(4));
        }
    }

    let mut reasons = serde_json::from_str::<Vec<String>>(&stored.reasons).unwrap_or_default();
    let mut risk = stored.risk.clone();
    let mut score = stored.score;
    let scan_source = "virustotal".to_string();
    let Some(stats) = stats else {
        reasons.push(
            "VirusTotal is still analyzing this file. Scan it again in a minute to see the result."
                .to_string(),
        );
        store_scan(
            &database,
            stored.id,
            &stored.detected_mime,
            &stored.kind,
            &risk,
            score,
            &reasons,
            stored.sha256.as_deref(),
            stored.content_available,
            &scan_source,
        )?;
        return Ok(view_of(&load_attachment(&database, stored.id)?));
    };

    let malicious = stats.get("malicious").and_then(|value| value.as_i64()).unwrap_or(0);
    let suspicious = stats
        .get("suspicious")
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let harmless = stats.get("harmless").and_then(|value| value.as_i64()).unwrap_or(0);
    let undetected = stats
        .get("undetected")
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let engines = malicious + suspicious + harmless + undetected;
    if malicious > 0 {
        risk = "malicious".to_string();
        score = 100;
        reasons.push(format!(
            "VirusTotal: {malicious} of {engines} engines flagged this file as malicious."
        ));
    } else if suspicious > 0 {
        if risk == "safe" {
            risk = "suspicious".to_string();
        }
        score = score.max(50);
        reasons.push(format!(
            "VirusTotal: {suspicious} of {engines} engines flagged this file as suspicious."
        ));
    } else {
        reasons.push(format!(
            "VirusTotal reported no detections across {engines} engines."
        ));
    }
    store_scan(
        &database,
        stored.id,
        &stored.detected_mime,
        &stored.kind,
        &risk,
        score,
        &reasons,
        stored.sha256.as_deref(),
        stored.content_available,
        &scan_source,
    )?;
    Ok(view_of(&load_attachment(&database, stored.id)?))
}

#[tauri::command]
pub fn get_attachment_content(app: AppHandle, id: i64) -> Result<AttachmentContent, String> {
    let database = storage::connection(&app)?;
    let stored = load_attachment(&database, id)?;
    if stored.size_bytes > MAX_DOWNLOAD_BYTES {
        return Err("This attachment is too large to export from the app.".to_string());
    }
    let bytes = stored
        .content
        .as_deref()
        .filter(|bytes| !bytes.is_empty())
        .ok_or_else(|| {
            "This attachment has no stored content. Sync the message from your provider first."
                .to_string()
        })?;
    Ok(AttachmentContent {
        id: stored.id,
        name: stored.name,
        mime_type: stored.mime_type,
        detected_mime: stored.detected_mime,
        size_bytes: stored.size_bytes,
        data_base64: STANDARD.encode(bytes),
    })
}
