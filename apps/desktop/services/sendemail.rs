use lettre::message::Mailbox;
use lettre::{Message, SmtpTransport, Transport};
use rusqlite::params;
use serde::Serialize;
use tauri::AppHandle;

use crate::mailbox;
use crate::storage;

#[derive(Debug, Serialize)]
pub struct SendEmail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[tauri::command]
pub fn list_sent_emails(app: AppHandle) -> Result<Vec<SendEmail>, String> {
    let database = storage::connection(&app)?;
    let mut statement = database
        .prepare(
            "SELECT e.recipient, e.subject, e.body_text
             FROM emails e JOIN folders f ON f.id = e.folder_id
             WHERE lower(f.name) = 'sent' AND e.deleted_at IS NULL
             ORDER BY e.received_at DESC, e.id DESC",
        )
        .map_err(|error| error.to_string())?;
    let emails = statement
        .query_map(params![], |row| {
            Ok(SendEmail {
                to: row.get(0)?,
                subject: row.get(1)?,
                body: row.get(2)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(emails)
}

fn required_setting(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("SMTP setting {name} is not configured"))
}

#[tauri::command(async)]
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
        .subject(subject.clone())
        .body(body.clone())
        .map_err(|error| error.to_string())?;
    let credentials = lettre::transport::smtp::authentication::Credentials::new(username, password);
    let mailer = SmtpTransport::starttls_relay(&host)
        .map_err(|error| error.to_string())?
        .port(port)
        .credentials(credentials)
        .build();

    mailer.send(&message).map_err(|error| error.to_string())?;
    mailbox::record_sent_email(&app, &to, &subject, &body)
}
