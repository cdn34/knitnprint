use std::{env, process::ExitCode};

use knitnprint_api::{config::Environment, object_storage::ObjectStorage};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    match cleanup().await {
        Ok((removed, failed)) => {
            println!("removed {removed} media assets; {failed} require retry");
            if failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("failed to clean abandoned media: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn cleanup() -> Result<(u64, u64), String> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let max_age_hours = parse_max_age(env::var("MEDIA_PENDING_MAX_HOURS").ok())?;
    let environment = Environment::from_env().map_err(|error| error.to_string())?;
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .map_err(|error| format!("database connection failed: {error}"))?;
    let storage = ObjectStorage::from_env(environment).await?;

    knitnprint_api::media_cleanup::cleanup_media(&pool, &storage, max_age_hours)
        .await
        .map_err(|error| format!("media cleanup failed: {error}"))
}

fn parse_max_age(value: Option<String>) -> Result<i32, String> {
    match value {
        Some(value) => match value.parse::<i32>() {
            Ok(hours @ 1..=168) => Ok(hours),
            _ => Err("MEDIA_PENDING_MAX_HOURS must be between 1 and 168".into()),
        },
        None => Ok(24),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_max_age;

    #[test]
    fn cleanup_age_defaults_and_stays_bounded() {
        assert_eq!(parse_max_age(None), Ok(24));
        assert_eq!(parse_max_age(Some("1".into())), Ok(1));
        assert!(parse_max_age(Some("0".into())).is_err());
        assert!(parse_max_age(Some("169".into())).is_err());
    }
}
