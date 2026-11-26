//! Read-only application diagnostics and release notes for the in-app
//! "Changes" and "About" views.

use serde::Serialize;
use tauri::AppHandle;

use crate::{mailbox, secrets, storage};

#[derive(Debug, Clone, Serialize)]
pub struct ChangeEntry {
    pub version: &'static str,
    pub date: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppDiagnostics {
    pub app_version: &'static str,
    pub engine: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub sqlite_version: String,
    pub database_path: String,
    pub database_bytes: i64,
    pub mailbox: mailbox::MailboxStats,
    pub cloud_configured: bool,
    pub keychain_available: bool,
}

#[tauri::command]
pub fn get_changelog() -> Vec<ChangeEntry> {
    vec![
        ChangeEntry {
            version: "0.4.0",
            date: "2026-11-26",
            title: "Local diagnostics",
            detail: "Redacted error events are stored in SQLite, rotated automatically, and can be exported as a JSON report. Nothing is uploaded automatically.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.4.0",
            date: "2026-11-26",
            title: "Advanced search",
            detail: "Filter the local mailbox by sender, recipient, subject, folder, date range, priority, security verdict, reputation and flags.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.4.0",
            date: "2026-11-26",
            title: "Offline outbox",
            detail: "Failed sends stay in a local outbox with retry, discard and an explicit sync status. Work-offline mode keeps everything on this device.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.4.0",
            date: "2026-11-26",
            title: "Accessibility polish",
            detail: "Added a skip link, visible focus rings, labelled advanced filters, a keyboard-trapped confirmation dialog and a global error boundary.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.3.0",
            date: "2026-11-14",
            title: "OS keychain secrets",
            detail: "Elyra Cloud sessions and IMAP passwords are now stored in the Windows Credential Manager through keyring-rs instead of SQLite.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.3.0",
            date: "2026-11-14",
            title: "IMAP provider sync",
            detail: "Connect Gmail, Outlook, iCloud, Yahoo, Fastmail or a custom IMAP server, test the connection, and pull the newest INBOX messages into the Rust mailbox.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.3.0",
            date: "2026-11-15",
            title: "First-run registration",
            detail: "Register once, choose your Elyra server URL, and continue offline if you prefer. The server URL can be changed later under Settings.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.3.0",
            date: "2026-11-15",
            title: "Send to any address",
            detail: "Registered users can send through the optional server SMTP relay when no local mail account is configured; replies return to the sender.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.3.0",
            date: "2026-11-15",
            title: "Server deployment",
            detail: "Production Compose stack, health-checked deploy script, optional automatic HTTPS with Caddy, and an opt-in GitHub Actions SSH deploy workflow.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.2.0",
            date: "2026-11-14",
            title: "Rust mailbox core",
            detail: "Filtering, search, folders, drafts, bulk actions, export and mailbox statistics now run in the Rust core instead of the browser.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.2.0",
            date: "2026-11-14",
            title: "Presentable interface refresh",
            detail: "Rebuilt the desktop workspace with a consistent SVG icon system, richer themes, accent colours, density controls, a command palette, and a calmer reading pane.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.2.0",
            date: "2026-11-14",
            title: "Installer and release pipeline",
            detail: "Added a dedicated MSI release workflow, stable download asset names, a Rust launcher that can download, verify and install an MSI, and a refreshed public website.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.1.2",
            date: "2026-09-28",
            title: "Accounts and notifications",
            detail: "Added account registration, login, authenticated notification delivery and the in-app notification inbox.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.1.1",
            date: "2026-09-20",
            title: "Security lab",
            detail: "Added SQLite posture checks, a bounded local audit trail and the first Rust AES-256-GCM encryption round-trip panel.",
            status: "ready",
        },
        ChangeEntry {
            version: "0.1.0",
            date: "2026-09-15",
            title: "First local workspace",
            detail: "Started with the privacy-first inbox prototype, realistic local mail, folders, message viewer and responsive layout.",
            status: "shipped",
        },
        ChangeEntry {
            version: "next",
            date: "planned",
            title: "Provider OAuth and background sync",
            detail: "Gmail and Outlook OAuth with PKCE plus scheduled background sync are the next provider milestones.",
            status: "next",
        },
    ]
}

#[tauri::command]
pub fn get_app_diagnostics(app: AppHandle) -> Result<AppDiagnostics, String> {
    let database = storage::connection(&app)?;
    let sqlite_version: String = database
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let database_path = storage::database_path(&app)?;
    let database_bytes = std::fs::metadata(&database_path)
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0);

    Ok(AppDiagnostics {
        app_version: env!("CARGO_PKG_VERSION"),
        engine: "Rust + Tauri + SQLite",
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        sqlite_version,
        database_path: database_path.to_string_lossy().to_string(),
        database_bytes,
        mailbox: mailbox::stats(&app)?,
        cloud_configured: crate::cloud::api_base(&app)
            .map(|url| url != "http://127.0.0.1:8787")
            .unwrap_or(false),
        keychain_available: secrets::available(),
    })
}
