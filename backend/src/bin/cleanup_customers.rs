use std::{env, process::ExitCode};

use knitprint_api::customer_retention::cleanup_expired_customer_data;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    match cleanup().await {
        Ok(summary) => {
            println!(
                "anonymized {} customers; removed {} addresses, {} accounts, {} sessions, {} stale login-limit buckets, and {} expired or used account tokens",
                summary.customers_anonymized,
                summary.addresses_removed,
                summary.accounts_removed,
                summary.sessions_removed,
                summary.login_rate_limits_removed,
                summary.account_tokens_removed,
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to clean customer data: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn cleanup() -> Result<knitprint_api::customer_retention::CleanupSummary, String> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let batch_size = parse_batch_size(env::var("CUSTOMER_CLEANUP_BATCH_SIZE").ok())?;
    let session_retention_days =
        parse_session_retention(env::var("CUSTOMER_SESSION_RETENTION_DAYS").ok())?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .map_err(|error| format!("database connection failed: {error}"))?;
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .map_err(|error| format!("database migration failed: {error}"))?;
    cleanup_expired_customer_data(&pool, batch_size, session_retention_days)
        .await
        .map_err(|error| format!("customer cleanup transaction failed: {error}"))
}

fn parse_batch_size(value: Option<String>) -> Result<i64, String> {
    parse_bounded(value, 100, 1, 1_000, "CUSTOMER_CLEANUP_BATCH_SIZE")
}

fn parse_session_retention(value: Option<String>) -> Result<i32, String> {
    parse_bounded(value, 7, 1, 365, "CUSTOMER_SESSION_RETENTION_DAYS")
}

fn parse_bounded<T>(
    value: Option<String>,
    default: T,
    minimum: T,
    maximum: T,
    name: &str,
) -> Result<T, String>
where
    T: std::str::FromStr + PartialOrd + Copy + std::fmt::Display,
{
    match value {
        Some(value) => match value.parse::<T>() {
            Ok(parsed) if parsed >= minimum && parsed <= maximum => Ok(parsed),
            _ => Err(format!("{name} must be between {minimum} and {maximum}")),
        },
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_batch_size, parse_session_retention};

    #[test]
    fn cleanup_configuration_defaults_and_stays_bounded() {
        assert_eq!(parse_batch_size(None), Ok(100));
        assert_eq!(parse_batch_size(Some("1".into())), Ok(1));
        assert!(parse_batch_size(Some("1001".into())).is_err());
        assert_eq!(parse_session_retention(None), Ok(7));
        assert_eq!(parse_session_retention(Some("365".into())), Ok(365));
        assert!(parse_session_retention(Some("0".into())).is_err());
    }
}
