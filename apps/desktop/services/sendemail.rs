use lettre::message::Mailbox;
use lettre::{Message, SmtpTransport, Transport};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize)]
pub struct SendEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

fn local_save_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join("elyra.sqlite3"))
}

fn send_email_connection(app: &AppHandle) -> Result<Connection, String> {
    let path = local_save_path(app)?;
    Connection::open(path).map_err(|error| error.to_string())
}

fn save_send_email(app: &AppHandle, email: &SendEmail) -> Result<(), String> {
    let conn = send_email_connection(app)?;
    conn.execute(
        "INSERT INTO send_email (recipient, subject, body) VALUES (?1, ?2, ?3)",
        params![email.to, email.subject, email.body],
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_sent_emails(app: AppHandle) -> Result<Vec<SendEmail>, String> {
    let conn = send_email_connection(&app)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS send_email (
            id INTEGER PRIMARY KEY,
            recipient TEXT NOT NULL,
            subject TEXT NOT NULL,
            body TEXT NOT NULL
        )",
        [],
    )
    .map_err(|error| error.to_string())?;
    let mut stmt = conn
        .prepare("SELECT recipient, subject, body FROM send_email ORDER BY id DESC")
        .map_err(|error| error.to_string())?;
    let email_iter = stmt
        .query_map([], |row| {
            Ok(SendEmail {
                to: row.get(0)?,
                subject: row.get(1)?,
                body: row.get(2)?,
            })
        })
        .map_err(|error| error.to_string())?;

    let mut emails = Vec::new();
    for email in email_iter {
        emails.push(email.map_err(|error| error.to_string())?);
    }
    Ok(emails)
}

fn required_setting(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("SMTP setting {name} is not configured"))
}

#[tauri::command]
pub fn send_email(app: AppHandle, to: String, subject: String, body: String) -> Result<(), String> {
    let recipient: Mailbox = to
        .parse()
        .map_err(|_| "Invalid recipient email address".to_string())?;
    let from: Mailbox = required_setting("ELYRA_SMTP_FROM")?
        .parse()
        .map_err(|_| "Invalid ELYRA_SMTP_FROM email address".to_string())?;
    let host = required_setting("ELYRA_SMTP_HOST")?;
    let username = required_setting("ELYRA_SMTP_USERNAME")?;
    let password = required_setting("ELYRA_SMTP_PASSWORD")?;
    let port = std::env::var("ELYRA_SMTP_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(587);

    let message = Message::builder()
        .from(from)
        .to(recipient)
        .subject(&subject)
        .body(body.clone())
        .map_err(|error| error.to_string())?;
    let credentials = lettre::transport::smtp::authentication::Credentials::new(username, password);
    let mailer = SmtpTransport::starttls_relay(&host)
        .map_err(|error| error.to_string())?
        .port(port)
        .credentials(credentials)
        .build();

    mailer.send(&message).map_err(|error| error.to_string())?;
    save_send_email(&app, &SendEmail { to, subject, body })
}