use std::{env, process::ExitCode};

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("DATABASE_URL is required");
        return ExitCode::FAILURE;
    };
    let retention_days = match env::var("SESSION_RETENTION_DAYS") {
        Ok(value) => match value.parse::<i32>() {
            Ok(days @ 1..=365) => days,
            _ => {
                eprintln!("SESSION_RETENTION_DAYS must be between 1 and 365");
                return ExitCode::FAILURE;
            }
        },
        Err(_) => 7,
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

    let result = sqlx::query(
        r#"
        DELETE FROM staff_sessions
        WHERE expires_at <= now()
           OR revoked_at <= now() - make_interval(days => $1)
        "#,
    )
    .bind(retention_days)
    .execute(&pool)
    .await;
    let attempts = sqlx::query(
        "DELETE FROM auth_login_rate_limits WHERE auth_scope = 'staff' AND updated_at < now() - interval '24 hours'",
    )
    .execute(&pool)
    .await;

    match (result, attempts) {
        (Ok(sessions), Ok(attempts)) => {
            println!(
                "removed {} sessions and {} stale login-attempt records",
                sessions.rows_affected(),
                attempts.rows_affected()
            );
            ExitCode::SUCCESS
        }
        (Err(error), _) | (_, Err(error)) => {
            eprintln!("failed to clean authentication records: {error}");
            ExitCode::FAILURE
        }
    }
}
