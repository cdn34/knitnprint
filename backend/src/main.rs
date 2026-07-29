use std::{net::SocketAddr, time::Duration};

use knitprint_api::{
    AppState, app,
    config::{Config, Environment},
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    init_tracing();

    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("invalid configuration: {error}");
        std::process::exit(2);
    });
    let database = connect_database(config.database_url.as_deref()).await;

    if config.environment == Environment::Production && database.is_none() {
        eprintln!("database connection is required in production");
        std::process::exit(2);
    }

    let address = SocketAddr::from((config.host, config.port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("API address should be available");

    info!(%address, environment = ?config.environment, "KnitPrint API listening");
    axum::serve(listener, app(AppState { database }))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("API server should run");
}

async fn connect_database(url: Option<&str>) -> Option<PgPool> {
    let Some(url) = url else {
        warn!("DATABASE_URL is not set; readiness will report unavailable");
        return None;
    };

    match PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(url)
        .await
    {
        Ok(pool) => {
            if let Err(error) = sqlx::migrate!("../migrations").run(&pool).await {
                warn!(%error, "database migrations failed");
                return None;
            }
            Some(pool)
        }
        Err(error) => {
            warn!(%error, "database unavailable at startup");
            None
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("knitprint_api=info,tower_http=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutdown signal received");
}
