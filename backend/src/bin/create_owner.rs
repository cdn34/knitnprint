use std::{env, process::ExitCode};

use knitprint_api::auth::hash_password;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::main]
async fn main() -> ExitCode {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("DATABASE_URL is required");
        return ExitCode::FAILURE;
    };
    let Some(email) = env::var("OWNER_EMAIL").ok() else {
        eprintln!("OWNER_EMAIL is required");
        return ExitCode::FAILURE;
    };
    let Some(password) = env::var("OWNER_PASSWORD").ok() else {
        eprintln!("OWNER_PASSWORD is required");
        return ExitCode::FAILURE;
    };
    let display_name = env::var("OWNER_NAME").unwrap_or_else(|_| "Store owner".into());
    if password.len() < 12 {
        eprintln!("OWNER_PASSWORD must be at least 12 characters");
        return ExitCode::FAILURE;
    }

    let pool = match PgPoolOptions::new().connect(&database_url).await {
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
    let password_hash = match hash_password(&password) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("failed to hash password: {error}");
            return ExitCode::FAILURE;
        }
    };
    let id = Uuid::now_v7();

    match sqlx::query(
        r#"
        INSERT INTO staff_users (id, email, password_hash, role, display_name)
        VALUES ($1, $2, $3, 'owner', $4)
        ON CONFLICT (email) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(email.trim().to_lowercase())
    .bind(password_hash)
    .bind(display_name.trim())
    .execute(&pool)
    .await
    {
        Ok(result) if result.rows_affected() == 1 => {
            println!("owner created");
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("an owner with that email already exists");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("failed to create owner: {error}");
            ExitCode::FAILURE
        }
    }
}
