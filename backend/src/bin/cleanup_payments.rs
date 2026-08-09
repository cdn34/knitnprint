use std::env;

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("DATABASE_URL is required");
        std::process::exit(2);
    });
    let batch_size = env::var("PAYMENT_CLEANUP_BATCH")
        .ok()
        .map(|value| value.parse::<i64>())
        .transpose()
        .unwrap_or_else(|_| {
            eprintln!("PAYMENT_CLEANUP_BATCH must be an integer between 1 and 1000");
            std::process::exit(2);
        })
        .unwrap_or(100);
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .unwrap_or_else(|error| {
            eprintln!("database unavailable: {error}");
            std::process::exit(1);
        });
    let cleaned = knitprint_api::payments::cleanup_abandoned(&pool, batch_size)
        .await
        .unwrap_or_else(|error| {
            eprintln!("payment cleanup failed: {error}");
            std::process::exit(1);
        });
    println!("cleaned {cleaned} abandoned payment(s)");
}
