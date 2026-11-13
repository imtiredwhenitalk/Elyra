mod cloud;
mod crypto;
mod diagnostics;
mod mailbox;
mod security;
mod storage;
#[path = "../../services/sendemail.rs"]
mod sendemail;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            storage::initialize(app.handle()).expect("failed to initialize Elyra local storage");
            mailbox::initialize(app.handle()).expect("failed to initialize Elyra mailbox engine");
            security::initialize(app.handle()).expect("failed to initialize Elyra security controls");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            storage::initialize_local_storage,
            storage::cache_email,
            storage::list_cached_emails,
            storage::set_local_setting,
            storage::get_local_settings,
            storage::set_local_settings,
            storage::delete_local_setting,
            mailbox::mailbox_stats,
            mailbox::seed_demo_mailbox,
            mailbox::reset_demo_mailbox,
            mailbox::refresh_mailbox,
            mailbox::list_mailbox,
            mailbox::get_message,
            mailbox::update_message,
            mailbox::delete_message,
            mailbox::bulk_update_messages,
            mailbox::mark_all_read,
            mailbox::empty_trash,
            mailbox::save_draft,
            mailbox::delete_draft,
            mailbox::search_suggestions,
            mailbox::export_mailbox,
            security::get_security_status,
            security::list_audit_events,
            crypto::generate_encryption_key,
            crypto::encrypt_text,
            crypto::decrypt_text,
            crypto::fingerprint,
            sendemail::send_email,
            sendemail::list_sent_emails,
            diagnostics::get_changelog,
            diagnostics::get_app_diagnostics,
            cloud::cloud_register,
            cloud::cloud_login,
            cloud::cloud_session,
            cloud::cloud_logout,
            cloud::cloud_list_notifications,
            cloud::cloud_send_notification,
            cloud::cloud_mark_notification_read,
            cloud::cloud_sync_cursor,
            cloud::cloud_update_cursor,
            cloud::cloud_health
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
