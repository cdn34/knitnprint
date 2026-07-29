use std::{env, process::ExitCode};

use serde_json::json;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL is required to seed the database");
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

    if let Err(error) = sqlx::migrate!("../migrations").run(&pool).await {
        eprintln!("failed to run migrations: {error}");
        return ExitCode::FAILURE;
    }

    if let Err(error) = sqlx::query(
        r#"
        INSERT INTO app_metadata (key, value)
        VALUES ('development_seed', $1)
        ON CONFLICT (key) DO UPDATE
        SET value = EXCLUDED.value, updated_at = now()
        "#,
    )
    .bind(json!({ "version": 1 }))
    .execute(&pool)
    .await
    {
        eprintln!("failed to record seed version: {error}");
        return ExitCode::FAILURE;
    }

    println!("database migrated and foundation seed applied");
    ExitCode::SUCCESS
}
