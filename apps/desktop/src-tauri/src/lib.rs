mod storage;
mod crypto;
mod security;
#[path = "../../services/sendemail.rs"]
mod sendemail;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let storage_initializer = |app: &tauri::AppHandle| {
        storage::initialize(app).expect("failed to initialize Elyra local storage");
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            storage_initializer(app.handle());
            security::initialize(app.handle()).expect("failed to initialize Elyra security controls");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            storage::initialize_local_storage,
            storage::cache_email,
            storage::list_cached_emails,
            storage::set_local_setting,
            security::get_security_status,
            security::list_audit_events,
            crypto::generate_encryption_key,
            crypto::encrypt_text,
            crypto::decrypt_text,
            crypto::fingerprint,
            sendemail::send_email,
            sendemail::list_sent_emails
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
