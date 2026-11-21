use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Updater, UpdaterExt};

/// Tauri/minisign public key for Elyra update artifacts.
///
/// This is intentionally kept in Rust rather than in `tauri.conf.json`; the
/// configuration file only carries the endpoint and an empty placeholder so
/// the plugin can deserialize its configuration.
pub const UPDATE_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQyREE1QzczREYzNDdCQkEKUldTNmV6VGZjMXphUXV3aEJQcU9IQVArb1NZTEdaQ0FVOUYrQ056LzJVODYydHh0aVdZR0ZudEEK";

const AUTO_CHECK_KEY: &str = "updates_auto_check";
const NOTIFY_KEY: &str = "updates_notify_available";
const LAST_CHECK_KEY: &str = "updates_last_checked_at";
const LAST_RESULT_KEY: &str = "updates_last_result";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

static CHECKING: AtomicBool = AtomicBool::new(false);
static INSTALLING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub status: String,
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub published_at: Option<String>,
    pub notes: Option<String>,
    pub target: Option<String>,
    pub checked_at: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferences {
    pub auto_check: bool,
    pub notify_available: bool,
    pub last_checked_at: Option<String>,
    pub last_result: Option<UpdateStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub stage: String,
    pub version: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: Option<f64>,
}

impl Default for UpdatePreferences {
    fn default() -> Self {
        Self {
            auto_check: true,
            notify_available: true,
            last_checked_at: None,
            last_result: None,
        }
    }
}

struct BusyGuard(&'static AtomicBool);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn now() -> String {
    chrono::Utc::now()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn current_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

fn read_setting(app: &AppHandle, key: &str) -> Option<String> {
    let database = crate::storage::connection(app).ok()?;
    database
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok()
}

fn write_setting(app: &AppHandle, key: &str, value: String) -> Result<(), String> {
    let database = crate::storage::connection(app)?;
    database
        .execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn bool_setting(raw: Option<String>, default: bool) -> bool {
    raw.map(|value| value == "true").unwrap_or(default)
}

fn status(
    value: &str,
    available: bool,
    app: &AppHandle,
    message: Option<String>,
) -> UpdateStatus {
    UpdateStatus {
        status: value.to_string(),
        available,
        current_version: current_version(app),
        latest_version: None,
        published_at: None,
        notes: None,
        target: None,
        checked_at: now(),
        message,
    }
}

pub fn read_preferences(app: &AppHandle) -> Result<UpdatePreferences, String> {
    let last_result = read_setting(app, LAST_RESULT_KEY)
        .and_then(|raw| serde_json::from_str::<UpdateStatus>(&raw).ok());

    Ok(UpdatePreferences {
        auto_check: bool_setting(read_setting(app, AUTO_CHECK_KEY), true),
        notify_available: bool_setting(read_setting(app, NOTIFY_KEY), true),
        last_checked_at: read_setting(app, LAST_CHECK_KEY),
        last_result,
    })
}

fn persist_status(app: &AppHandle, value: &UpdateStatus) {
    let _ = write_setting(app, LAST_CHECK_KEY, value.checked_at.clone());
    if let Ok(raw) = serde_json::to_string(value) {
        let _ = write_setting(app, LAST_RESULT_KEY, raw);
    }
}

fn build_updater(app: &AppHandle) -> Result<Updater, String> {
    app.updater_builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| error.to_string())
}

async fn perform_check(app: &AppHandle) -> UpdateStatus {
    let updater = match build_updater(app) {
        Ok(updater) => updater,
        Err(error) => {
            return status(
                "error",
                false,
                app,
                Some(format!("Updates could not be initialized: {error}")),
            )
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            let latest_version = update.version.clone();
            let published_at = update
                .raw_json
                .get("pub_date")
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
                .or_else(|| update.date.as_ref().map(ToString::to_string));

            UpdateStatus {
                status: "available".to_string(),
                available: true,
                current_version: current_version(app),
                latest_version: Some(latest_version.clone()),
                published_at,
                notes: update.body.clone(),
                target: Some(update.target.clone()),
                checked_at: now(),
                message: Some(format!("Version {latest_version} is ready to install.")),
            }
        }
        Ok(None) => status(
            "upToDate",
            false,
            app,
            Some("You are running the latest available version.".to_string()),
        ),
        Err(error) => status(
            "error",
            false,
            app,
            Some(format!("Could not check for updates: {error}")),
        ),
    }
}

pub fn initialize(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let preferences = read_preferences(&handle).unwrap_or_default();
        if !preferences.auto_check {
            return;
        }
        if CHECKING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        let _guard = BusyGuard(&CHECKING);

        let result = perform_check(&handle).await;
        persist_status(&handle, &result);
        if result.available && preferences.notify_available {
            let _ = handle.emit("elyra-update-available", &result);
        }
    });
}

#[tauri::command]
pub fn get_update_preferences(app: AppHandle) -> Result<UpdatePreferences, String> {
    read_preferences(&app)
}

#[tauri::command]
pub fn set_update_preferences(
    app: AppHandle,
    auto_check: Option<bool>,
    notify_available: Option<bool>,
) -> Result<UpdatePreferences, String> {
    if let Some(value) = auto_check {
        write_setting(&app, AUTO_CHECK_KEY, value.to_string())?;
    }
    if let Some(value) = notify_available {
        write_setting(&app, NOTIFY_KEY, value.to_string())?;
    }
    read_preferences(&app)
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateStatus, String> {
    if CHECKING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(status(
            "busy",
            false,
            &app,
            Some("An update check is already running.".to_string()),
        ));
    }
    let _guard = BusyGuard(&CHECKING);

    let result = perform_check(&app).await;
    persist_status(&app, &result);
    Ok(result)
}

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<serde_json::Value, String> {
    if INSTALLING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("An update installation is already in progress.".to_string());
    }
    let _guard = BusyGuard(&INSTALLING);

    let version = current_version(&app);
    let _ = app.emit(
        "elyra-update-progress",
        UpdateProgress {
            stage: "preparing".to_string(),
            version: version.clone(),
            downloaded: 0,
            total: None,
            percent: None,
        },
    );

    let updater = build_updater(&app)?;
    let mut update = updater
        .check()
        .await
        .map_err(|error| format!("Could not check for updates: {error}"))?
        .ok_or_else(|| "No update is available to install.".to_string())?;
    update.timeout = Some(REQUEST_TIMEOUT);
    let latest_version = update.version.clone();

    let downloaded = Arc::new(AtomicU64::new(0));
    let progress_app = app.clone();
    let progress_app_finish = app.clone();
    let progress_version = latest_version.clone();
    let progress_version_finish = latest_version.clone();
    let progress_counter = downloaded.clone();
    let bytes = update
        .download(
            |chunk, total| {
                let received = progress_counter
                    .fetch_add(chunk as u64, Ordering::SeqCst)
                    .saturating_add(chunk as u64);
                let percent = total.map(|total| {
                    if total == 0 {
                        100.0
                    } else {
                        ((received as f64 / total as f64) * 100.0).min(100.0)
                    }
                });
                let _ = progress_app.emit(
                    "elyra-update-progress",
                    UpdateProgress {
                        stage: "downloading".to_string(),
                        version: progress_version.clone(),
                        downloaded: received,
                        total,
                        percent,
                    },
                );
            },
            move || {
                let received = downloaded.load(Ordering::SeqCst);
                let _ = progress_app_finish.emit(
                    "elyra-update-progress",
                    UpdateProgress {
                        stage: "verified".to_string(),
                        version: progress_version_finish,
                        downloaded: received,
                        total: None,
                        percent: Some(100.0),
                    },
                );
            },
        )
        .await
        .map_err(|error| format!("Update download or signature verification failed: {error}"))?;

    let _ = app.emit(
        "elyra-update-progress",
        UpdateProgress {
            stage: "installing".to_string(),
            version: latest_version.clone(),
            downloaded: 0,
            total: None,
            percent: None,
        },
    );

    update.install(&bytes).map_err(|error| {
        format!("Update verification or installation failed: {error}. Your existing installation was not changed.")
    })?;

    Ok(json!({
        "installed": true,
        "version": latest_version,
        "restartRecommended": cfg!(windows),
    }))
}
