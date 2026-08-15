use std::{env, process::ExitCode};

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    let Some(database_url) = env::var("MIGRATION_DATABASE_URL")
        .ok()
        .or_else(|| env::var("DATABASE_URL").ok())
    else {
        eprintln!(
            "MIGRATION_DATABASE_URL (or DATABASE_URL for development) is required to migrate the database"
        );
        return ExitCode::FAILURE;
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

    match sqlx::migrate!("../migrations").run(&pool).await {
        Ok(()) => {
            println!("database migrations applied");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to run migrations: {error}");
            ExitCode::FAILURE
        }
    }
}
