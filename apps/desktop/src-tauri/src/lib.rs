mod cloud;
mod crypto;
mod diagnostics;
mod integrations;
mod keys;
mod mailbox;
mod providers;
mod secrets;
mod security;
mod shield;
mod storage;
mod threats;
#[path = "../../services/sendemail.rs"]
mod sendemail;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            storage::initialize(app.handle()).expect("failed to initialize Elyra local storage");
            mailbox::initialize(app.handle()).expect("failed to initialize Elyra mailbox engine");
            providers::initialize(app.handle()).expect("failed to initialize Elyra provider accounts");
            security::initialize(app.handle()).expect("failed to initialize Elyra security controls");
            threats::initialize(app.handle()).expect("failed to initialize Elyra threat intelligence");
            integrations::initialize(app.handle()).expect("failed to initialize Elyra integrations");
            shield::initialize(app.handle()).expect("failed to initialize Elyra Shield");
            keys::initialize(app.handle()).expect("failed to initialize Elyra key management");
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
            integrations::list_integrations,
            integrations::connect_integration,
            integrations::test_integration,
            integrations::set_integration_enabled,
            integrations::remove_integration_key,
            threats::scan_links,
            shield::analyze_message,
            shield::get_message_security,
            shield::list_security_alerts,
            shield::get_shield_summary,
            shield::report_false_positive,
            keys::list_crypto_keys,
            keys::generate_crypto_key,
            keys::rotate_crypto_key,
            keys::revoke_crypto_key,
            keys::export_public_key,
            keys::import_public_key,
            keys::list_crypto_contacts,
            keys::delete_crypto_contact,
            crypto::generate_encryption_key,
            crypto::encrypt_text,
            crypto::decrypt_text,
            crypto::fingerprint,
            sendemail::send_email,
            sendemail::list_sent_emails,
            providers::provider_presets,
            providers::list_email_accounts,
            providers::save_email_account,
            providers::delete_email_account,
            providers::test_email_account,
            providers::test_email_settings,
            providers::sync_email_account,
            providers::sync_all_accounts,
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
            cloud::cloud_health,
            cloud::cloud_api_url,
            cloud::cloud_set_api_url,
            cloud::cloud_mail_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
