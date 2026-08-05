use std::{env, process::ExitCode};

use knitprint_api::carts::cleanup_expired;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("DATABASE_URL is required");
        return ExitCode::FAILURE;
    };
    let batch_size = match env::var("CART_CLEANUP_BATCH_SIZE") {
        Ok(value) => match value.parse::<i64>() {
            Ok(size @ 1..=1_000) => size,
            _ => {
                eprintln!("CART_CLEANUP_BATCH_SIZE must be between 1 and 1000");
                return ExitCode::FAILURE;
            }
        },
        Err(_) => 100,
    };
    let pool = match PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("failed to connect to PostgreSQL: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = sqlx::migrate!("../migrations").run(&pool).await {
        eprintln!("failed to run migrations: {error}");
        return ExitCode::FAILURE;
    }
    match cleanup_expired(&pool, batch_size).await {
        Ok(removed) => {
            println!("removed {removed} expired carts");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to clean expired carts: {error}");
            ExitCode::FAILURE
        }
    }
}
