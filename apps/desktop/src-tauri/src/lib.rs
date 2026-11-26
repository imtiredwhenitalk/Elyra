mod attachments;
mod cloud;
mod crypto;
mod diagnostics;
mod integrations;
mod keys;
mod mailbox;
mod priority;
mod providers;
mod reporting;
mod reputation;
mod search;
mod secrets;
mod security;
#[path = "../../services/sendemail.rs"]
mod sendemail;
mod senders;
mod shield;
mod storage;
mod sync;
mod threats;
mod updates;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_updater::Builder::new()
                .pubkey(updates::UPDATE_PUBLIC_KEY)
                .build(),
        )
        .setup(|app| {
            storage::initialize(app.handle()).expect("failed to initialize Elyra local storage");
            reporting::initialize(app.handle()).expect("failed to initialize Elyra diagnostics");
            updates::initialize(app.handle());
            senders::initialize(app.handle()).expect("failed to initialize Elyra sender rules");
            mailbox::initialize(app.handle()).expect("failed to initialize Elyra mailbox engine");
            attachments::initialize(app.handle())
                .expect("failed to initialize Elyra attachment scanning");
            priority::initialize(app.handle()).expect("failed to initialize Elyra priority engine");
            providers::initialize(app.handle())
                .expect("failed to initialize Elyra provider accounts");
            sync::initialize(app.handle()).expect("failed to initialize Elyra sync state");
            security::initialize(app.handle())
                .expect("failed to initialize Elyra security controls");
            threats::initialize(app.handle())
                .expect("failed to initialize Elyra threat intelligence");
            integrations::initialize(app.handle())
                .expect("failed to initialize Elyra integrations");
            shield::initialize(app.handle()).expect("failed to initialize Elyra Shield");
            reputation::initialize(app.handle())
                .expect("failed to initialize Elyra sender reputation");
            keys::initialize(app.handle()).expect("failed to initialize Elyra key management");
            let _ = reporting::record(
                app.handle(),
                "info",
                "app",
                "app.start",
                "Elyra desktop started",
            );
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
            reporting::log_diagnostic_event,
            reporting::list_diagnostic_events,
            reporting::export_diagnostic_report,
            reporting::get_crash_reporting_status,
            reporting::set_crash_reporting_enabled,
            reporting::get_diagnostic_logging_status,
            reporting::set_diagnostic_logging_enabled,
            reporting::clear_diagnostic_log,
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
            search::advanced_search,
            sync::queue_outgoing_message,
            sync::list_outgoing_queue,
            sync::flush_outgoing_queue,
            sync::retry_outgoing_message,
            sync::discard_outgoing_message,
            sync::get_sync_status,
            sync::set_offline_mode,
            attachments::scan_attachment,
            attachments::scan_all_attachments,
            attachments::scan_attachment_external,
            attachments::get_attachment_content,
            priority::refresh_priority_suggestions,
            senders::list_sender_rules,
            senders::save_sender_rule,
            senders::delete_sender_rule,
            senders::apply_sender_rules,
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
            reputation::get_sender_reputation,
            reputation::refresh_domain_reputation,
            reputation::report_reputation_issue,
            keys::list_crypto_keys,
            keys::generate_crypto_key,
            keys::rotate_crypto_key,
            keys::revoke_crypto_key,
            keys::export_public_key,
            keys::import_public_key,
            keys::list_crypto_contacts,
            keys::verify_crypto_contact,
            keys::revoke_crypto_contact,
            keys::delete_crypto_contact,
            keys::import_crypto_encryption_public_key,
            keys::list_crypto_encryption_keys,
            keys::generate_crypto_encryption_key,
            keys::rotate_crypto_encryption_key,
            keys::revoke_crypto_encryption_key,
            keys::export_crypto_encryption_public_key,
            crypto::generate_encryption_key,
            crypto::encrypt_text,
            crypto::decrypt_text,
            crypto::encrypt_text_for_recipient,
            crypto::seal_message_envelope,
            crypto::open_message_envelope,
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
            updates::get_update_preferences,
            updates::set_update_preferences,
            updates::check_for_updates,
            updates::download_and_install_update,
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
