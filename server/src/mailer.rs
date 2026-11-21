//! Optional server-side SMTP relay.
//!
//! Registered Elyra users can send mail through the server when no local SMTP
//! account is configured. The relay is entirely opt-in: without `ELYRA_SMTP_HOST`
//! the service reports itself as unconfigured and the endpoint returns a clear
//! error instead of pretending the message was delivered.

use std::env;

use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MailError {
    #[error("the Elyra mail relay is not configured on this server")]
    NotConfigured,
    #[error("invalid recipient email address")]
    InvalidRecipient,
    #[error("invalid reply-to email address")]
    InvalidReplyTo,
    #[error("mail configuration error: {0}")]
    Config(String),
    #[error("could not build the message: {0}")]
    Build(String),
    #[error("could not send the message: {0}")]
    Send(String),
}

#[derive(Debug, Clone)]
struct MailConfig {
    host: String,
    port: u16,
    security: String,
    username: String,
    password: String,
    from: Mailbox,
}

#[derive(Debug, Clone)]
pub struct MailService {
    config: Option<MailConfig>,
}

fn normalize_security(raw: &str) -> String {
    if raw.trim().eq_ignore_ascii_case("tls") {
        "tls".to_string()
    } else {
        "starttls".to_string()
    }
}

fn port_for_security(security: &str) -> u16 {
    if security == "tls" { 465 } else { 587 }
}

impl MailService {
    pub fn from_env() -> Result<Self, MailError> {
        let host = env::var("ELYRA_SMTP_HOST")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let Some(host) = host else {
            return Ok(Self { config: None });
        };

        let from_raw = env::var("ELYRA_SMTP_FROM").map_err(|_| {
            MailError::Config("ELYRA_SMTP_FROM is required when ELYRA_SMTP_HOST is set".into())
        })?;
        let mut from: Mailbox = from_raw
            .trim()
            .parse()
            .map_err(|_| MailError::Config("ELYRA_SMTP_FROM is not a valid email address".into()))?;
        if let Ok(name) = env::var("ELYRA_SMTP_FROM_NAME") {
            let name = name.trim().to_string();
            if !name.is_empty() {
                from.name = Some(name);
            }
        }

        let security = normalize_security(
            &env::var("ELYRA_SMTP_SECURITY").unwrap_or_else(|_| "starttls".to_string()),
        );
        let default_port = port_for_security(&security);
        let port = env::var("ELYRA_SMTP_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(default_port);
        let username = env::var("ELYRA_SMTP_USERNAME").unwrap_or_default();
        let password = env::var("ELYRA_SMTP_PASSWORD").unwrap_or_default();

        Ok(Self {
            config: Some(MailConfig {
                host,
                port,
                security,
                username,
                password,
                from,
            }),
        })
    }

    /// A relay that is explicitly disabled, for tests and local tooling.
    pub(crate) fn unconfigured() -> Self {
        Self { config: None }
    }

    pub fn configured(&self) -> bool {
        self.config.is_some()
    }

    pub fn from_address(&self) -> Option<String> {
        self.config
            .as_ref()
            .map(|config| config.from.email.to_string())
    }

    pub async fn send(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        reply_to: Option<(String, String)>,
    ) -> Result<(), MailError> {
        let Some(config) = self.config.clone() else {
            return Err(MailError::NotConfigured);
        };
        let recipient: Mailbox = to
            .trim()
            .parse()
            .map_err(|_| MailError::InvalidRecipient)?;
        let subject = subject.trim().to_string();
        let body = body.to_string();

        tokio::task::spawn_blocking(move || {
            let builder = if config.security == "tls" {
                SmtpTransport::relay(&config.host)
            } else {
                SmtpTransport::starttls_relay(&config.host)
            }
            .map_err(|error| MailError::Config(error.to_string()))?;

            let mut builder = builder.port(config.port);
            if !config.username.trim().is_empty() {
                builder = builder.credentials(Credentials::new(
                    config.username.clone(),
                    config.password.clone(),
                ));
            }
            let transport = builder.build();
            let mut message = Message::builder()
                .from(config.from.clone())
                .to(recipient)
                .subject(subject);
            if let Some((email, name)) = reply_to {
                let address: lettre::Address = email
                    .parse()
                    .map_err(|_| MailError::InvalidReplyTo)?;
                let name = if name.trim().is_empty() {
                    None
                } else {
                    Some(name)
                };
                message = message.reply_to(Mailbox::new(name, address));
            }
            let message = message
                .body(body)
                .map_err(|error| MailError::Build(error.to_string()))?;
            transport
                .send(&message)
                .map(|_| ())
                .map_err(|error| MailError::Send(error.to_string()))
        })
        .await
        .map_err(|error| MailError::Send(error.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_modes_normalize_to_supported_values() {
        assert_eq!(normalize_security(" TLS "), "tls");
        assert_eq!(normalize_security("starttls"), "starttls");
        assert_eq!(normalize_security("something-else"), "starttls");
        assert_eq!(port_for_security("tls"), 465);
        assert_eq!(port_for_security("starttls"), 587);
    }

    #[tokio::test]
    async fn disabled_relay_fails_closed_without_network_access() {
        let mail = MailService::unconfigured();
        assert!(!mail.configured());
        assert_eq!(mail.from_address(), None);
        let error = mail
            .send("to@example.test", "Subject", "Body", None)
            .await
            .expect_err("a disabled relay must not send mail");
        assert!(matches!(error, MailError::NotConfigured));
    }
}
