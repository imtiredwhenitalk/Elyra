//! Email provider accounts, IMAP synchronisation and SMTP settings.
//!
//! Account metadata lives in SQLite. Passwords live in the OS keychain through
//! [`crate::secrets`]. Synchronisation pulls the newest messages from an IMAP
//! INBOX and upserts them into the local mailbox without overwriting local
//! read/starred state.

use chrono::{DateTime, Utc};
use imap::types::Flag;
use lettre::transport::smtp::authentication::Credentials;
use lettre::SmtpTransport;
use mail_parser::MessageParser;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{mailbox, secrets, shield, storage};

type ImapSession = imap::Session<imap::Connection>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailAccount {
    pub id: i64,
    pub provider: String,
    pub email: String,
    pub display_name: String,
    pub username: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_security: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_security: String,
    pub has_password: bool,
    pub auth_method: String,
    pub oauth_connected: bool,
    pub last_sync_at: Option<String>,
    pub sync_error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailAccountInput {
    pub id: Option<i64>,
    pub provider: String,
    pub email: String,
    pub display_name: String,
    pub username: String,
    pub password: Option<String>,
    pub imap_host: String,
    pub imap_port: Option<u16>,
    pub imap_security: String,
    pub smtp_host: String,
    pub smtp_port: Option<u16>,
    pub smtp_security: String,
    pub auth_method: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub imap_host: &'static str,
    pub imap_port: u16,
    pub imap_security: &'static str,
    pub smtp_host: &'static str,
    pub smtp_port: u16,
    pub smtp_security: &'static str,
    pub hint: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTest {
    pub imap_ok: bool,
    pub smtp_ok: bool,
    pub detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncAccountResult {
    pub account_id: i64,
    pub email: String,
    pub imported: i64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummary {
    pub accounts: Vec<SyncAccountResult>,
    pub imported: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub security: String,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
}

const PROVIDERS: &[ProviderPreset] = &[
    ProviderPreset {
        id: "gmail",
        label: "Gmail",
        imap_host: "imap.gmail.com",
        imap_port: 993,
        imap_security: "tls",
        smtp_host: "smtp.gmail.com",
        smtp_port: 465,
        smtp_security: "tls",
        hint: "Use a Google app password when 2-step verification is enabled.",
    },
    ProviderPreset {
        id: "outlook",
        label: "Outlook / Microsoft 365",
        imap_host: "outlook.office365.com",
        imap_port: 993,
        imap_security: "tls",
        smtp_host: "smtp.office365.com",
        smtp_port: 587,
        smtp_security: "starttls",
        hint: "Microsoft may require an app password or OAuth-enabled mailbox.",
    },
    ProviderPreset {
        id: "icloud",
        label: "iCloud Mail",
        imap_host: "imap.mail.me.com",
        imap_port: 993,
        imap_security: "tls",
        smtp_host: "smtp.mail.me.com",
        smtp_port: 587,
        smtp_security: "starttls",
        hint: "Generate an app-specific password in your Apple ID settings.",
    },
    ProviderPreset {
        id: "yahoo",
        label: "Yahoo Mail",
        imap_host: "imap.mail.yahoo.com",
        imap_port: 993,
        imap_security: "tls",
        smtp_host: "smtp.mail.yahoo.com",
        smtp_port: 465,
        smtp_security: "tls",
        hint: "Yahoo requires an app password for third-party clients.",
    },
    ProviderPreset {
        id: "fastmail",
        label: "Fastmail",
        imap_host: "imap.fastmail.com",
        imap_port: 993,
        imap_security: "tls",
        smtp_host: "smtp.fastmail.com",
        smtp_port: 465,
        smtp_security: "tls",
        hint: "Create an app password in Fastmail settings.",
    },
    ProviderPreset {
        id: "proton",
        label: "Proton Mail",
        imap_host: "127.0.0.1",
        imap_port: 1143,
        imap_security: "starttls",
        smtp_host: "127.0.0.1",
        smtp_port: 1025,
        smtp_security: "starttls",
        hint: "Proton requires Proton Mail Bridge running on this device.",
    },
    ProviderPreset {
        id: "custom",
        label: "Custom IMAP / SMTP",
        imap_host: "",
        imap_port: 993,
        imap_security: "tls",
        smtp_host: "",
        smtp_port: 465,
        smtp_security: "tls",
        hint: "Enter the server names, ports and security mode from your provider.",
    },
];

pub(crate) fn now_string() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn normalize_security(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "starttls" => "starttls".to_string(),
        _ => "tls".to_string(),
    }
}

fn normalize_port(value: Option<u16>, fallback: u16) -> u16 {
    match value {
        Some(port) if port > 0 => port,
        _ => fallback,
    }
}

fn row_to_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<EmailAccount> {
    Ok(EmailAccount {
        id: row.get(0)?,
        provider: row.get(1)?,
        email: row.get(2)?,
        display_name: row.get(3)?,
        username: row.get(4)?,
        imap_host: row.get(5)?,
        imap_port: row.get::<_, i64>(6)? as u16,
        imap_security: row.get(7)?,
        smtp_host: row.get(8)?,
        smtp_port: row.get::<_, i64>(9)? as u16,
        smtp_security: row.get(10)?,
        has_password: false,
        auth_method: row.get(13)?,
        oauth_connected: false,
        last_sync_at: row.get(11)?,
        sync_error: row.get(12)?,
    })
}

const ACCOUNT_COLUMNS: &str = "id, provider, email, display_name, username, imap_host, imap_port,
     imap_security, smtp_host, smtp_port, smtp_security, last_sync_at, sync_error, auth_method";

pub(crate) fn initialize(app: &AppHandle) -> Result<(), String> {
    let database = storage::connection(app)?;
    for (column, definition) in [
        ("username", "TEXT NOT NULL DEFAULT ''"),
        ("imap_host", "TEXT NOT NULL DEFAULT ''"),
        ("imap_port", "INTEGER NOT NULL DEFAULT 993"),
        ("imap_security", "TEXT NOT NULL DEFAULT 'tls'"),
        ("smtp_host", "TEXT NOT NULL DEFAULT ''"),
        ("smtp_port", "INTEGER NOT NULL DEFAULT 465"),
        ("smtp_security", "TEXT NOT NULL DEFAULT 'tls'"),
        ("last_sync_at", "TEXT"),
        ("sync_error", "TEXT"),
        ("auth_method", "TEXT NOT NULL DEFAULT 'password'"),
        ("updated_at", "TEXT NOT NULL DEFAULT ''"),
    ] {
        mailbox::ensure_column(&database, "accounts", column, definition)?;
    }
    database
        .execute(
            "UPDATE accounts SET username = email WHERE username = '' OR username IS NULL",
            [],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn provider_presets() -> Vec<ProviderPreset> {
    PROVIDERS
        .iter()
        .map(|preset| ProviderPreset { ..*preset })
        .collect()
}

#[tauri::command]
pub fn list_email_accounts(app: AppHandle) -> Result<Vec<EmailAccount>, String> {
    let database = storage::connection(&app)?;
    let mut statement = database
        .prepare(&format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts ORDER BY id"
        ))
        .map_err(|error| error.to_string())?;
    let mut accounts = statement
        .query_map([], row_to_account)
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    for account in &mut accounts {
        account.has_password = secrets::get_secret(&secrets::account_user(account.id))
            .ok()
            .flatten()
            .is_some();
        account.oauth_connected = secrets::get_secret(&secrets::oauth_user(account.id))
            .ok()
            .flatten()
            .is_some();
    }
    Ok(accounts)
}

fn account_by_id(app: &AppHandle, id: i64) -> Result<EmailAccount, String> {
    let database = storage::connection(app)?;
    let mut account = database
        .query_row(
            &format!("SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE id = ?1"),
            params![id],
            row_to_account,
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Email account not found".to_string())?;
    account.has_password = secrets::get_secret(&secrets::account_user(account.id))
        .ok()
        .flatten()
        .is_some();
    account.oauth_connected = secrets::get_secret(&secrets::oauth_user(account.id))
        .ok()
        .flatten()
        .is_some();
    Ok(account)
}

#[tauri::command]
pub fn save_email_account(
    app: AppHandle,
    account: EmailAccountInput,
) -> Result<EmailAccount, String> {
    let email = account.email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.len() < 3 {
        return Err("Enter a valid email address".to_string());
    }
    let provider = if account.provider.trim().is_empty() {
        "custom".to_string()
    } else {
        account.provider.trim().to_ascii_lowercase()
    };
    let display_name = if account.display_name.trim().is_empty() {
        email.clone()
    } else {
        account.display_name.trim().to_string()
    };
    let username = if account.username.trim().is_empty() {
        email.clone()
    } else {
        account.username.trim().to_string()
    };
    let imap_security = normalize_security(&account.imap_security);
    let smtp_security = normalize_security(&account.smtp_security);
    let imap_port = normalize_port(
        account.imap_port,
        if imap_security == "starttls" {
            143
        } else {
            993
        },
    );
    let smtp_port = normalize_port(
        account.smtp_port,
        if smtp_security == "starttls" {
            587
        } else {
            465
        },
    );
    let imap_host = account.imap_host.trim().to_string();
    let smtp_host = account.smtp_host.trim().to_string();
    let password = account
        .password
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let existing_id = account.id;
    let has_stored_password = existing_id
        .and_then(|id| {
            secrets::get_secret(&secrets::account_user(id))
                .ok()
                .flatten()
        })
        .is_some();
    let has_oauth_session = existing_id
        .and_then(|id| secrets::get_secret(&secrets::oauth_user(id)).ok().flatten())
        .is_some();
    let keep_oauth =
        account.auth_method.as_deref() == Some("oauth2") && password.is_none() && has_oauth_session;
    let auth_method = if keep_oauth { "oauth2" } else { "password" };
    if auth_method == "password"
        && password.is_none()
        && !has_stored_password
        && !imap_host.is_empty()
    {
        return Err("Enter the IMAP password for this account".to_string());
    }
    let database = storage::connection(&app)?;
    let updated_at = now_string();

    let id = match account.id {
        Some(id) => {
            let changed = database
                .execute(
                    "UPDATE accounts SET provider = ?2, email = ?3, display_name = ?4,
                        username = ?5, imap_host = ?6, imap_port = ?7, imap_security = ?8,
                        smtp_host = ?9, smtp_port = ?10, smtp_security = ?11,
                        auth_method = ?12, updated_at = ?13
                     WHERE id = ?1",
                    params![
                        id,
                        provider,
                        email,
                        display_name,
                        username,
                        imap_host,
                        imap_port as i64,
                        imap_security,
                        smtp_host,
                        smtp_port as i64,
                        smtp_security,
                        auth_method,
                        updated_at
                    ],
                )
                .map_err(|error| error.to_string())?;
            if changed == 0 {
                return Err("Email account not found".to_string());
            }
            id
        }
        None => {
            database
                .execute(
                    "INSERT INTO accounts (provider, email, display_name, username, imap_host,
                        imap_port, imap_security, smtp_host, smtp_port, smtp_security,
                        auth_method, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        provider,
                        email,
                        display_name,
                        username,
                        imap_host,
                        imap_port as i64,
                        imap_security,
                        smtp_host,
                        smtp_port as i64,
                        smtp_security,
                        auth_method,
                        updated_at
                    ],
                )
                .map_err(|error| error.to_string())?;
            database.last_insert_rowid()
        }
    };

    if let Some(password) = password {
        if let Err(error) = secrets::set_secret(&secrets::account_user(id), password) {
            if existing_id.is_none() {
                let _ = database.execute("DELETE FROM accounts WHERE id = ?1", params![id]);
            }
            return Err(error);
        }
    }
    if auth_method == "password" && has_oauth_session {
        secrets::delete_secret(&secrets::oauth_user(id))?;
    }

    account_by_id(&app, id)
}

#[tauri::command]
pub fn delete_email_account(app: AppHandle, id: i64) -> Result<(), String> {
    secrets::delete_secret(&secrets::account_user(id))?;
    secrets::delete_secret(&secrets::oauth_user(id))?;
    let database = storage::connection(&app)?;
    database
        .execute("DELETE FROM accounts WHERE id = ?1", params![id])
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// SASL XOAUTH2 initial client response (RFC 7628 §3.1). The IMAP crate
/// base64-encodes the bytes returned by `process`.
struct XOAuth2 {
    email: String,
    token: String,
}

impl imap::Authenticator for XOAuth2 {
    type Response = String;

    fn process(&self, _challenge: &[u8]) -> Self::Response {
        format!(
            "user={}\u{1}auth=Bearer {}\u{1}\u{1}",
            self.email, self.token
        )
    }
}

fn connect_imap(
    app: &AppHandle,
    account: &EmailAccount,
    password: &str,
) -> Result<ImapSession, String> {
    let host = account.imap_host.trim();
    if host.is_empty() {
        return Err("This account has no IMAP server configured".to_string());
    }
    let oauth = account.auth_method == "oauth2";
    if !oauth && password.is_empty() {
        return Err("No IMAP password is stored for this account".to_string());
    }
    // imap 3 builds TLS itself from the port and security mode. Keeping the
    // connector here would duplicate certificate verification and pin the
    // session type to a removed transport alias.
    let mut builder = imap::ClientBuilder::new(host, account.imap_port);
    if account.imap_security == "starttls" {
        builder = builder.mode(imap::ConnectionMode::StartTls);
    }
    let client = builder.connect().map_err(|error| {
        format!(
            "Could not reach {host}:{} over IMAP: {error}",
            account.imap_port
        )
    })?;
    if oauth {
        let token = crate::oauth::access_token(app, account.id)?;
        client
            .authenticate(
                "XOAUTH2",
                &XOAuth2 {
                    // RFC 7628 expects the authorized mailbox identity. The
                    // editable username is for password logins; OAuth accounts
                    // must keep using the address stored at connection time.
                    email: if oauth {
                        account.email.trim().to_string()
                    } else {
                        account.username.trim().to_string()
                    },
                    token,
                },
            )
            .map_err(|(error, _)| format!("IMAP OAuth sign-in failed: {error}"))
    } else {
        client
            .login(account.username.trim(), password)
            .map_err(|(error, _)| format!("IMAP sign-in failed: {error}"))
    }
}

pub(crate) fn smtp_config(app: &AppHandle, account_id: i64) -> Result<SmtpConfig, String> {
    let account = account_by_id(app, account_id)?;
    if account.smtp_host.trim().is_empty() {
        return Err("This account has no SMTP server configured".to_string());
    }
    if account.auth_method == "oauth2" {
        return Err(
            "OAuth is enabled for IMAP only. Add an app password for SMTP, or send through the Elyra relay."
                .to_string(),
        );
    }
    let password = secrets::get_secret(&secrets::account_user(account_id))?
        .ok_or_else(|| "No password is stored for this account".to_string())?;
    Ok(SmtpConfig {
        host: account.smtp_host,
        port: account.smtp_port,
        security: account.smtp_security,
        username: account.username,
        password,
        from_email: account.email,
        from_name: account.display_name,
    })
}

pub(crate) fn smtp_transport(config: &SmtpConfig) -> Result<SmtpTransport, String> {
    let credentials = Credentials::new(config.username.clone(), config.password.clone());
    let builder = if config.security == "starttls" {
        SmtpTransport::starttls_relay(&config.host)
    } else {
        SmtpTransport::relay(&config.host)
    }
    .map_err(|error| format!("Could not initialize SMTP for {}: {error}", config.host))?;
    Ok(builder.port(config.port).credentials(credentials).build())
}

/// Exercise the real SMTP transport (TLS/STARTTLS, EHLO and NOOP) instead of
/// only opening a TCP socket, which produced false positives for any listening
/// service on the configured port.
fn smtp_ready(account: &EmailAccount, password: &str) -> Result<(), String> {
    let host = account.smtp_host.trim();
    if host.is_empty() {
        return Err("SMTP is not configured".to_string());
    }
    if account.auth_method == "oauth2" {
        return Err("OAuth SMTP needs an app password or the Elyra relay".to_string());
    }
    let config = SmtpConfig {
        host: host.to_string(),
        port: account.smtp_port,
        security: account.smtp_security.clone(),
        username: account.username.clone(),
        password: password.to_string(),
        from_email: account.email.clone(),
        from_name: account.display_name.clone(),
    };
    let transport = smtp_transport(&config)?;
    match transport.test_connection() {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("SMTP server {host} rejected the connection")),
        Err(error) => Err(format!("SMTP connection to {host} failed: {error}")),
    }
}

fn test_settings(app: &AppHandle, account: &EmailAccount, password: &str) -> AccountTest {
    let mut detail = Vec::new();
    let imap_ok = match connect_imap(app, account, password) {
        Ok(mut session) => match session.select("INBOX") {
            Ok(mailbox) => {
                detail.push(format!(
                    "IMAP inbox reachable ({} messages)",
                    mailbox.exists
                ));
                let _ = session.logout();
                true
            }
            Err(error) => {
                detail.push(format!("IMAP connected but INBOX failed: {error}"));
                let _ = session.logout();
                false
            }
        },
        Err(error) => {
            detail.push(error);
            false
        }
    };

    let smtp_ok = match smtp_ready(account, password) {
        Ok(()) => {
            detail.push("SMTP server accepted the connection".to_string());
            true
        }
        Err(error) => {
            detail.push(error);
            false
        }
    };

    AccountTest {
        imap_ok,
        smtp_ok,
        detail: detail.join(" · "),
    }
}

#[tauri::command(async)]
pub fn test_email_account(app: AppHandle, account_id: i64) -> Result<AccountTest, String> {
    let account = account_by_id(&app, account_id)?;
    let password = if account.auth_method == "oauth2" {
        String::new()
    } else {
        secrets::get_secret(&secrets::account_user(account_id))?
            .ok_or_else(|| "No password is stored for this account".to_string())?
    };
    Ok(test_settings(&app, &account, &password))
}

#[tauri::command(async)]
pub fn test_email_settings(
    app: AppHandle,
    account: EmailAccountInput,
) -> Result<AccountTest, String> {
    let oauth = account.auth_method.as_deref() == Some("oauth2")
        || account
            .id
            .and_then(|id| account_by_id(&app, id).ok())
            .map(|account| account.auth_method == "oauth2")
            .unwrap_or(false);
    let password = if oauth {
        String::new()
    } else {
        match account
            .password
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(password) => password.to_string(),
            None => match account.id {
                Some(id) => secrets::get_secret(&secrets::account_user(id))?
                    .ok_or_else(|| "Enter a password to test this account".to_string())?,
                None => return Err("Enter a password to test this account".to_string()),
            },
        }
    };
    let imap_security = normalize_security(&account.imap_security);
    let smtp_security = normalize_security(&account.smtp_security);
    let transient = EmailAccount {
        id: account.id.unwrap_or(0),
        provider: account.provider,
        email: account.email.trim().to_ascii_lowercase(),
        display_name: account.display_name,
        username: if account.username.trim().is_empty() {
            account.email.trim().to_ascii_lowercase()
        } else {
            account.username.trim().to_string()
        },
        imap_host: account.imap_host.trim().to_string(),
        imap_port: normalize_port(
            account.imap_port,
            if imap_security == "starttls" {
                143
            } else {
                993
            },
        ),
        imap_security,
        smtp_host: account.smtp_host.trim().to_string(),
        smtp_port: normalize_port(
            account.smtp_port,
            if smtp_security == "starttls" {
                587
            } else {
                465
            },
        ),
        smtp_security,
        has_password: !password.is_empty(),
        auth_method: if oauth { "oauth2" } else { "password" }.to_string(),
        oauth_connected: oauth,
        last_sync_at: None,
        sync_error: None,
    };
    Ok(test_settings(&app, &transient, &password))
}

fn strip_html(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut inside_tag = false;
    for character in html.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                output.push(' ');
            }
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }
    output
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn sync_inbox(
    app: &AppHandle,
    account: &EmailAccount,
    password: &str,
    limit: u32,
) -> Result<i64, String> {
    let mut session = connect_imap(app, account, password)?;
    let mailbox_info = session
        .select("INBOX")
        .map_err(|error| format!("Could not open the IMAP inbox: {error}"))?;
    if mailbox_info.exists == 0 {
        let _ = session.logout();
        return Ok(0);
    }

    let start = mailbox_info.exists.saturating_sub(limit).max(1);
    let sequence = format!("{start}:{}", mailbox_info.exists);
    let fetches = session
        .fetch(sequence, "(UID FLAGS INTERNALDATE BODY.PEEK[])")
        .map_err(|error| format!("Could not fetch messages from the IMAP inbox: {error}"))?;
    let database = storage::connection(app)?;
    let inbox_folder = mailbox::folder_id(&database, "Inbox")?;
    let mut imported = 0_i64;

    for fetch in &fetches {
        let Some(raw) = fetch.body() else {
            continue;
        };
        let Some(parsed) = MessageParser::new().parse(raw) else {
            continue;
        };
        let uid = fetch.uid.unwrap_or(fetch.message);
        let provider_id = format!("imap-inbox-{uid}");
        let subject = parsed
            .subject()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("(no subject)")
            .to_string();
        let from_mailbox = parsed.from().and_then(|list| list.mailboxes().next());
        let sender = from_mailbox
            .and_then(|mailbox| {
                mailbox
                    .name()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .or_else(|| mailbox.address().map(str::to_string))
            })
            .unwrap_or_else(|| "Unknown sender".to_string());
        let sender_address = from_mailbox
            .and_then(|mailbox| mailbox.address())
            .map(|value| value.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let body_html = parsed.body_html(0).map(|value| value.into_owned());
        let body_text = parsed
            .body_text(0)
            .map(|value| value.into_owned())
            .or_else(|| body_html.as_deref().map(strip_html))
            .unwrap_or_default();
        let preview = mailbox::preview_of(&body_text);
        let received_at = parsed
            .date()
            .map(|date| {
                DateTime::<Utc>::from_timestamp(date.to_timestamp(), 0).unwrap_or_else(Utc::now)
            })
            .unwrap_or_else(Utc::now)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let flags = fetch.flags();
        let is_read = flags.iter().any(|flag| matches!(flag, Flag::Seen));
        let is_starred = flags.iter().any(|flag| matches!(flag, Flag::Flagged));
        let size_bytes = raw.len() as i64;
        let auth_results = crate::reputation::parse_auth_results(raw)
            .and_then(|auth| serde_json::to_string(&auth).ok())
            .unwrap_or_default();

        let inserted = database
            .execute(
                "INSERT INTO emails (account_id, provider_id, sender, sender_address, subject,
                    body_text, body_html, preview, folder_id, is_read, is_starred, received_at,
                    tag, recipient, has_attachments, updated_at, size_bytes, auth_results)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'Personal', ?13, 0, ?12, ?14, ?15)
                 ON CONFLICT(account_id, provider_id) DO NOTHING",
                params![
                    account.id,
                    provider_id,
                    sender,
                    sender_address,
                    subject,
                    body_text,
                    body_html,
                    preview,
                    inbox_folder,
                    if is_read { 1 } else { 0 },
                    if is_starred { 1 } else { 0 },
                    received_at,
                    account.email,
                    size_bytes,
                    auth_results
                ],
            )
            .map_err(|error| error.to_string())?;

        if inserted == 0 {
            database
                .execute(
                    "UPDATE emails SET sender = ?3, sender_address = ?4, subject = ?5,
                        body_text = ?6, body_html = ?7, preview = ?8, received_at = ?9,
                        updated_at = ?9, size_bytes = ?10, auth_results = ?11
                     WHERE account_id = ?1 AND provider_id = ?2",
                    params![
                        account.id,
                        provider_id,
                        sender,
                        sender_address,
                        subject,
                        body_text,
                        body_html,
                        preview,
                        received_at,
                        size_bytes,
                        auth_results
                    ],
                )
                .map_err(|error| error.to_string())?;
            continue;
        }

        imported += 1;
        let email_id = database.last_insert_rowid();
        for part in parsed.attachments() {
            let name = part.attachment_name().unwrap_or("attachment").to_string();
            let mime_type = part
                .content_type()
                .map(|content_type| match content_type.subtype() {
                    Some(subtype) => format!("{}/{}", content_type.ctype(), subtype),
                    None => content_type.ctype().to_string(),
                })
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let content = part.decoded();
            mailbox::insert_attachment_with_content(
                &database,
                email_id,
                &name,
                &mime_type,
                part.decoded_len() as i64,
                Some(content.as_ref()),
            )?;
        }
        // Classify attachments from their real bytes. ClamAV only runs when the
        // user has enabled it in Settings.
        let _ = crate::attachments::scan_email_attachments(&database, email_id);
        // Run the local Shield checks at import time. External link scans are
        // deferred until the message is opened so synchronisation stays fast.
        let _ = crate::priority::apply_auto_priority(&database, email_id, true);
        let _ = crate::senders::apply_to_email(&database, email_id);
        let _ = shield::analyze_email_local(&database, email_id);
    }

    let _ = session.logout();
    Ok(imported)
}

#[tauri::command(async)]
pub fn sync_email_account(
    app: AppHandle,
    account_id: i64,
    limit: Option<u32>,
) -> Result<SyncAccountResult, String> {
    let account = account_by_id(&app, account_id)?;
    let password = if account.auth_method == "oauth2" {
        String::new()
    } else {
        secrets::get_secret(&secrets::account_user(account_id))?
            .ok_or_else(|| "No password is stored for this account".to_string())?
    };
    let limit = limit.unwrap_or(40).clamp(1, 200);
    let outcome = sync_inbox(&app, &account, &password, limit);
    let database = storage::connection(&app)?;
    let synced_at = now_string();

    match outcome {
        Ok(imported) => {
            database
                .execute(
                    "UPDATE accounts SET last_sync_at = ?2, sync_error = NULL, updated_at = ?2
                     WHERE id = ?1",
                    params![account_id, synced_at],
                )
                .map_err(|error| error.to_string())?;
            database
                .execute(
                    "INSERT INTO settings (key, value) VALUES ('last_sync_at', ?1)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![synced_at],
                )
                .map_err(|error| error.to_string())?;
            Ok(SyncAccountResult {
                account_id,
                email: account.email,
                imported,
                error: None,
            })
        }
        Err(error) => {
            database
                .execute(
                    "UPDATE accounts SET sync_error = ?2, updated_at = ?3 WHERE id = ?1",
                    params![account_id, error, synced_at],
                )
                .map_err(|database_error| database_error.to_string())?;
            Err(error)
        }
    }
}

#[tauri::command(async)]
pub fn sync_all_accounts(app: AppHandle, limit: Option<u32>) -> Result<SyncSummary, String> {
    let accounts = list_email_accounts(app.clone())?;
    let mut results = Vec::new();
    let mut imported = 0_i64;
    for account in accounts {
        if account.imap_host.trim().is_empty() {
            continue;
        }
        match sync_email_account(app.clone(), account.id, limit) {
            Ok(result) => {
                imported += result.imported;
                results.push(result);
            }
            Err(error) => results.push(SyncAccountResult {
                account_id: account.id,
                email: account.email,
                imported: 0,
                error: Some(error),
            }),
        }
    }
    Ok(SyncSummary {
        accounts: results,
        imported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_and_ports_are_normalized() {
        assert_eq!(normalize_security("STARTTLS"), "starttls");
        assert_eq!(normalize_security("plain"), "tls");
        assert_eq!(normalize_port(Some(0), 993), 993);
        assert_eq!(normalize_port(Some(587), 465), 587);
    }

    #[test]
    fn html_fallback_is_readable() {
        assert_eq!(strip_html("<p>Hello&nbsp;<b>there</b></p>"), "Hello there");
    }

    #[test]
    fn presets_include_gmail_and_outlook() {
        assert!(PROVIDERS.iter().any(|preset| preset.id == "gmail"));
        assert!(PROVIDERS.iter().any(|preset| preset.id == "outlook"));
    }

    #[test]
    fn xoauth2_response_uses_rfc_7628_separators() {
        let auth = XOAuth2 {
            email: "user@example.test".to_string(),
            token: "token-value".to_string(),
        };
        assert_eq!(
            auth.process(&[]),
            "user=user@example.test\u{1}auth=Bearer token-value\u{1}\u{1}"
        );
    }
}
