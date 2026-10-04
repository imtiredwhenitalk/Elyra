mod api;
mod auth;
mod database;
mod sync;
mod users;

use std::{env, net::SocketAddr, sync::Arc};

use api::router;
use auth::AuthService;
use database::Database;
use sync::SyncService;
use users::UserService;

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub auth: AuthService,
    pub users: UserService,
    pub sync: SyncService,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG").unwrap_or_else(|_| "elyra_server=info,tower_http=info".into()),
        )
        .init();

    let database = Database::connect_from_env().await?;
    database.migrate().await?;
    let state = Arc::new(AppState {
        database: database.clone(),
        auth: AuthService::from_env()?,
        users: UserService::new(database.clone()),
        sync: SyncService::new(database.clone()),
    });
    let address: SocketAddr = env::var("ELYRA_SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8787".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "Elyra server listening");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}
