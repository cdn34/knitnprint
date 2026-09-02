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
    let config = Config::from_env().unwrap_or_else(|error| {
        eprintln!("invalid configuration: {error}");
        std::process::exit(2);
    });
    init_tracing(config.environment);
    let database = connect_database(
        config.database_url.as_deref(),
        config.environment == Environment::Production,
    )
    .await;
    let media_storage =
        knitprint_api::media::MediaStorage::from_env(config.environment == Environment::Production)
            .await
            .unwrap_or_else(|error| {
                eprintln!("invalid media storage configuration: {error}");
                std::process::exit(2);
            });
    let media_scanner = knitprint_api::media_scanner::MediaScanner::from_env(
        config.environment == Environment::Production,
    )
    .unwrap_or_else(|error| {
        eprintln!("invalid media scanner configuration: {error}");
        std::process::exit(2);
    });
    let email = knitprint_api::email::EmailService::from_env(config.environment)
        .await
        .unwrap_or_else(|error| {
            eprintln!("invalid email configuration: {error}");
            std::process::exit(2);
        });
    let payments = knitprint_api::payments::PaymentService::from_env(config.environment)
        .unwrap_or_else(|error| {
            eprintln!("invalid payment configuration: {error}");
            std::process::exit(2);
        });
    let packlink = knitprint_api::packlink::PacklinkService::from_env().unwrap_or_else(|error| {
        eprintln!("invalid Packlink configuration: {error}");
        std::process::exit(2);
    });

    if config.environment == Environment::Production && database.is_none() {
        eprintln!("database connection is required in production");
        std::process::exit(2);
    }

    if let Some(pool) = database.clone() {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                if let Err(error) = knitprint_api::discounts::expire_due(&pool).await {
                    warn!(%error, "automatic discount expiry failed");
                }
            }
        });
    }

    let address = SocketAddr::from((config.host, config.port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("API address should be available");

    info!(%address, environment = ?config.environment, "KnitPrint API listening");
    axum::serve(
        listener,
        app(AppState {
            database,
            media_storage,
            media_scanner,
            email,
            payments,
            packlink,
            trust_proxy_headers: config.trust_proxy_headers,
            secure_cookies: config.environment == Environment::Production,
            manual_payments_enabled: config.environment != Environment::Production,
            security: knitprint_api::security::SecurityPolicy {
                allowed_origins: config.web_origins,
                production: config.environment == Environment::Production,
            },
        })
        .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("API server should run");
}

async fn connect_database(url: Option<&str>, production: bool) -> Option<PgPool> {
    let Some(url) = url else {
        warn!("DATABASE_URL is not set; readiness will report unavailable");
        return None;
    };

    match PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .after_connect(move |connection, _| {
            Box::pin(async move {
                sqlx::query("SET application_name = 'knitprint-api'")
                    .execute(&mut *connection)
                    .await?;
                if production {
                    sqlx::query("SET statement_timeout = 15000")
                        .execute(&mut *connection)
                        .await?;
                    sqlx::query("SET lock_timeout = 5000")
                        .execute(&mut *connection)
                        .await?;
                    sqlx::query("SET idle_in_transaction_session_timeout = 15000")
                        .execute(&mut *connection)
                        .await?;
                }
                Ok(())
            })
        })
        .connect(url)
        .await
    {
        Ok(pool) => {
            if !production && let Err(error) = sqlx::migrate!("../migrations").run(&pool).await {
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

fn init_tracing(environment: Environment) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("knitprint_api=info,tower_http=info"));

    if environment == Environment::Production {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .compact()
            .init();
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutdown signal received");
}
