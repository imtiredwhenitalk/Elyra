use lettre::message::Mailbox;
use lettre::{Address, Message, SmtpTransport, Transport};
use rusqlite::params;
use serde::Serialize;
use tauri::AppHandle;

use crate::cloud;
use crate::mailbox;
use crate::providers;
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

fn env_transport() -> Result<(Mailbox, String, SmtpTransport), String> {
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
    let sender = from
        .name
        .clone()
        .unwrap_or_else(|| "Alex Smith".to_string());
    let credentials = lettre::transport::smtp::authentication::Credentials::new(username, password);
    let transport = SmtpTransport::starttls_relay(&host)
        .map_err(|error| error.to_string())?
        .port(port)
        .credentials(credentials)
        .build();
    Ok((from, sender, transport))
}

fn account_transport(
    app: &AppHandle,
    account_id: i64,
) -> Result<(Mailbox, String, SmtpTransport), String> {
    let config = providers::smtp_config(app, account_id)?;
    let address: Address = config
        .from_email
        .parse()
        .map_err(|_| "The account email address is invalid".to_string())?;
    let from = Mailbox::new(Some(config.from_name.clone()), address);
    let transport = providers::smtp_transport(&config)?;
    Ok((from, config.from_name, transport))
}

#[tauri::command(async)]
pub fn send_email(
    app: AppHandle,
    to: String,
    subject: String,
    body: String,
    account_id: Option<i64>,
) -> Result<(), String> {
    let recipient: Mailbox = to
        .parse()
        .map_err(|_| "Invalid recipient email address".to_string())?;

    if let Some(id) = account_id {
        let (from, sender, mailer) = account_transport(&app, id)?;
        let message = Message::builder()
            .from(from)
            .to(recipient)
            .subject(subject.clone())
            .body(body.clone())
            .map_err(|error| error.to_string())?;
        mailer.send(&message).map_err(|error| error.to_string())?;
        return mailbox::record_sent_email(&app, Some(id), &sender, &to, &subject, &body);
    }

    match env_transport() {
        Ok((from, sender, mailer)) => {
            let message = Message::builder()
                .from(from)
                .to(recipient)
                .subject(subject.clone())
                .body(body.clone())
                .map_err(|error| error.to_string())?;
            mailer.send(&message).map_err(|error| error.to_string())?;
            mailbox::record_sent_email(&app, None, &sender, &to, &subject, &body)
        }
        Err(env_error) => {
            // A fresh install has no SMTP environment. If the user is signed in,
            // send through the Elyra Cloud relay so mail still reaches external
            // addresses without extra configuration.
            match cloud::relay_send(&app, &to, &subject, &body) {
                Ok(sender) => {
                    mailbox::record_sent_email(&app, None, &sender, &to, &subject, &body)
                }
                Err(relay_error) => Err(format!(
                    "{env_error}. Elyra Cloud relay failed: {relay_error}"
                )),
            }
        }
    }
}
