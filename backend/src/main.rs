use std::{net::SocketAddr, time::Duration};

use knitnprint_api::{
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
    let deployed = config.environment.is_deployed();
    init_tracing(config.environment);
    let database = connect_database(config.database_url.as_deref(), deployed).await;
    let media_storage = knitnprint_api::object_storage::ObjectStorage::from_env(config.environment)
        .await
        .unwrap_or_else(|error| {
            eprintln!("invalid media storage configuration: {error}");
            std::process::exit(2);
        });
    let media_scanner = knitnprint_api::media_scanner::MediaScanner::from_env(deployed)
        .unwrap_or_else(|error| {
            eprintln!("invalid media scanner configuration: {error}");
            std::process::exit(2);
        });
    let email = knitnprint_api::email::EmailService::from_env(config.environment)
        .await
        .unwrap_or_else(|error| {
            eprintln!("invalid email configuration: {error}");
            std::process::exit(2);
        });
    let payments = knitnprint_api::payments::PaymentService::from_env(config.environment)
        .unwrap_or_else(|error| {
            eprintln!("invalid payment configuration: {error}");
            std::process::exit(2);
        });

    if deployed && database.is_none() {
        eprintln!("database connection is required in staging and production");
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

    info!(%address, environment = ?config.environment, "KnitNPrint API listening");
    axum::serve(
        listener,
        app(AppState {
            database,
            media_storage: Some(media_storage),
            media_scanner,
            email,
            payments,
            trusted_proxy_hops: config.trusted_proxy_hops,
            secure_cookies: deployed,
            manual_payments_enabled: !deployed,
            security: knitnprint_api::security::SecurityPolicy {
                allowed_origins: config.web_origins,
                deployed,
            },
        })
        .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("API server should run");
}

async fn connect_database(url: Option<&str>, deployed: bool) -> Option<PgPool> {
    let Some(url) = url else {
        warn!("DATABASE_URL is not set; readiness will report unavailable");
        return None;
    };

    match PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .after_connect(move |connection, _| {
            Box::pin(async move {
                sqlx::query("SET application_name = 'knitnprint-api'")
                    .execute(&mut *connection)
                    .await?;
                if deployed {
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
            if !deployed && let Err(error) = sqlx::migrate!("../migrations").run(&pool).await {
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
        .unwrap_or_else(|_| EnvFilter::new("knitnprint_api=info,tower_http=info"));

    if environment.is_deployed() {
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
