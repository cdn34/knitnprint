use std::{env, process::ExitCode};

use serde::Serialize;
use sqlx::postgres::PgPoolOptions;

#[derive(Serialize)]
struct OperationalCheck {
    status: &'static str,
    terminal_notification_failures: i64,
    stale_notification_claims: i64,
    overdue_payment_attempts: i64,
    stale_quarantined_media: i64,
    infected_media: i64,
    expired_carts: i64,
    expired_customer_records: i64,
}

#[derive(Serialize)]
struct OperationalError<'a> {
    status: &'static str,
    message: &'a str,
}

#[tokio::main]
async fn main() -> ExitCode {
    match check().await {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string(&report).expect("operational report should serialize")
            );
            if report.status == "ok" {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string(&OperationalError {
                    status: "error",
                    message: &error,
                })
                .expect("operational error should serialize")
            );
            ExitCode::FAILURE
        }
    }
}

async fn check() -> Result<OperationalCheck, String> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required")?;
    let media_hours = env::var("MEDIA_PENDING_MAX_HOURS")
        .ok()
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| (1..=168).contains(value))
        .unwrap_or(24);
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .map_err(|error| format!("database connection failed: {error}"))?;

    let (
        terminal_notification_failures,
        stale_notification_claims,
        overdue_payment_attempts,
        stale_quarantined_media,
        infected_media,
        expired_carts,
        expired_customer_records,
    ): (i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            (SELECT count(*) FROM notification_jobs WHERE status = 'failed'),
            (SELECT count(*) FROM notification_jobs
             WHERE status = 'processing' AND updated_at < now() - interval '15 minutes'),
            (SELECT count(*) FROM payment_attempts
             WHERE status IN ('creating','pending','processing')
               AND expires_at < now() - interval '1 hour'),
            (SELECT count(*) FROM media_assets
             WHERE status = 'pending' AND created_at < now() - make_interval(hours => $1)),
            (SELECT count(*) FROM media_assets WHERE scan_status = 'infected'),
            (SELECT count(*) FROM carts
             WHERE expires_at < now() - interval '24 hours'),
            (SELECT count(*) FROM customers
             WHERE anonymized_at IS NULL
               AND retention_expires_at < now() - interval '24 hours')
        "#,
    )
    .bind(media_hours)
    .fetch_one(&pool)
    .await
    .map_err(|error| format!("operational query failed: {error}"))?;
    let degraded = terminal_notification_failures
        + stale_notification_claims
        + overdue_payment_attempts
        + stale_quarantined_media
        + infected_media
        + expired_carts
        + expired_customer_records
        > 0;
    Ok(OperationalCheck {
        status: if degraded { "degraded" } else { "ok" },
        terminal_notification_failures,
        stale_notification_claims,
        overdue_payment_attempts,
        stale_quarantined_media,
        infected_media,
        expired_carts,
        expired_customer_records,
    })
}
