mod storage;

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            storage::initialize_local_storage,
            storage::cache_email,
            storage::list_cached_emails,
            storage::set_local_setting
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
