//! OS keychain-backed secret storage.
//!
//! Elyra Cloud sessions and IMAP passwords never live in SQLite. On Windows the
//! secrets are written to the Windows Credential Manager through `keyring-rs`.
//! The SQLite `settings.cloud_session` row is only read once to migrate sessions
//! created by older releases.

const SERVICE: &str = "com.elyra.desktop";
const CLOUD_SESSION_USER: &str = "cloud_session";

#[cfg(windows)]
mod platform {
    use super::SERVICE;

    fn entry(user: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, user)
            .map_err(|error| format!("Could not open the OS keychain: {error}"))
    }

    pub(crate) fn set_secret(user: &str, value: &str) -> Result<(), String> {
        entry(user)?
            .set_password(value)
            .map_err(|error| format!("Could not save the secret in the OS keychain: {error}"))
    }

    pub(crate) fn get_secret(user: &str) -> Result<Option<String>, String> {
        match entry(user)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!(
                "Could not read the secret from the OS keychain: {error}"
            )),
        }
    }

    pub(crate) fn delete_secret(user: &str) -> Result<(), String> {
        match entry(user)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(format!(
                "Could not delete the secret from the OS keychain: {error}"
            )),
        }
    }

    pub(crate) fn available() -> bool {
        match entry("availability-probe") {
            Ok(probe) => match probe.get_password() {
                Ok(_) | Err(keyring::Error::NoEntry) => true,
                Err(_) => false,
            },
            Err(_) => false,
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub(crate) fn set_secret(_user: &str, _value: &str) -> Result<(), String> {
        Err("The OS keychain is only available in the Windows desktop build".to_string())
    }

    pub(crate) fn get_secret(_user: &str) -> Result<Option<String>, String> {
        Err("The OS keychain is only available in the Windows desktop build".to_string())
    }

    pub(crate) fn delete_secret(_user: &str) -> Result<(), String> {
        Err("The OS keychain is only available in the Windows desktop build".to_string())
    }

    pub(crate) fn available() -> bool {
        false
    }
}

pub(crate) use platform::{available, delete_secret, get_secret, set_secret};

pub(crate) fn cloud_session_user() -> &'static str {
    CLOUD_SESSION_USER
}

pub(crate) fn account_user(account_id: i64) -> String {
    format!("imap-account-{account_id}")
}
