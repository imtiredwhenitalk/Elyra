//! Read-only application diagnostics and release notes for the in-app
//! "Changes" and "About" views.

use serde::Serialize;
use tauri::AppHandle;

use crate::{mailbox, storage};

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
}

#[tauri::command]
pub fn get_changelog() -> Vec<ChangeEntry> {
    vec![
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
            title: "Provider sync",
            detail: "Gmail and Outlook OAuth with PKCE, IMAP fetch, OS keychain token storage and background sync are next.",
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
        cloud_configured: std::env::var("ELYRA_API_URL").is_ok(),
    })
}
