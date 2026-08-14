use std::env;

use knitprint_api::{config::Environment, email::EmailService, notifications::deliver_due};
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
    let environment = match env::var("APP_ENV").as_deref() {
        Ok("production") => Environment::Production,
        Ok("test") => Environment::Test,
        _ => Environment::Development,
    };
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
    let summary = deliver_due(&pool, &email, batch_size)
        .await
        .unwrap_or_else(|error| {
            eprintln!("notification delivery failed: {error}");
            std::process::exit(1);
        });
    println!(
        "notification delivery complete: {} sent, {} failed",
        summary.sent, summary.failed
    );
}
