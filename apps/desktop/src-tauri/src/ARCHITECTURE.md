# Rust application architecture

Rust modules implement the Tauri entry point, command handlers, and local
storage. `mailbox.rs` owns mailbox queries and mutations, `cloud.rs` owns cloud
auth/notifications/sync, `diagnostics.rs` reports engine details, `security.rs`
owns the crypto lab, and `storage.rs` owns SQLite access. Keep filesystem,
database, and network operations on the Rust side.
