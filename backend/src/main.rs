mod error;
mod health;

use std::{env, net::SocketAddr, time::Duration};

use axum::{
    Router,
    http::{HeaderName, Method},
    routing::get,
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    database: Option<PgPool>,
}

#[tokio::main]
async fn main() {
    init_tracing();

    let database = connect_database().await;
    let state = AppState { database };
    let request_id = HeaderName::from_static("x-request-id");

    let app = Router::new()
        .route("/api/health", get(health::health))
        .route("/api/ready", get(health::ready))
        .fallback(error::not_found)
        .with_state(state)
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET])
                .allow_headers(Any),
        );

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);
    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("API address should be available");

    info!(%address, "KnitPrint API listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("API server should run");
}

async fn connect_database() -> Option<PgPool> {
    let Ok(url) = env::var("DATABASE_URL") else {
        warn!("DATABASE_URL is not set; readiness will report unavailable");
        return None;
    };

    match PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&url)
        .await
    {
        Ok(pool) => Some(pool),
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
