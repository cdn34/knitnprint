use std::env;

use knitnprint_api::{
    config::Environment, email::EmailService, notifications::deliver_due_with_storage,
    object_storage::ObjectStorage,
};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("DATABASE_URL is required");
        std::process::exit(2);
    });
    let batch_size = env::var("NOTIFICATION_BATCH_SIZE")
        .ok()
        .map(|value| value.parse::<i64>())
        .transpose()
        .unwrap_or_else(|_| {
            eprintln!("NOTIFICATION_BATCH_SIZE must be an integer between 1 and 100");
            std::process::exit(2);
        })
        .unwrap_or(25);
    let environment = Environment::from_env().unwrap_or_else(|error| {
        eprintln!("invalid configuration: {error}");
        std::process::exit(2);
    });
    let email = EmailService::from_env(environment)
        .await
        .unwrap_or_else(|error| {
            eprintln!("invalid email configuration: {error}");
            std::process::exit(2);
        });
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .unwrap_or_else(|error| {
            eprintln!("database unavailable: {error}");
            std::process::exit(1);
        });
    let storage = ObjectStorage::from_env(environment)
        .await
        .unwrap_or_else(|error| {
            eprintln!("invalid media storage configuration: {error}");
            std::process::exit(2);
        });
    let summary = deliver_due_with_storage(&pool, &email, batch_size, Some(&storage))
        .await
        .unwrap_or_else(|error| {
            eprintln!("notification delivery failed: {error}");
            std::process::exit(1);
        });
    println!(
        "notification delivery complete: {} sent, {} failed",
        summary.sent, summary.failed
    );
    let b2b_summary = knitnprint_api::b2b::deliver_due(&pool, &email, batch_size)
        .await
        .unwrap_or_else(|error| {
            eprintln!("B2B notification delivery failed: {error}");
            std::process::exit(1);
        });
    println!(
        "B2B notification delivery complete: {} sent, {} failed",
        b2b_summary.sent, b2b_summary.failed
    );
}
