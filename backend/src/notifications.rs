use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{email::EmailService, object_storage::ObjectStorage, orders::load_order};

const MAX_ATTEMPTS: i32 = 8;

#[derive(Clone, Debug, Serialize, ToSchema, FromRow)]
pub struct NotificationStatus {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub attempt_count: i32,
    pub last_error: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct DeliverySummary {
    pub sent: u64,
    pub failed: u64,
}

#[derive(FromRow)]
struct ClaimedNotification {
    id: Uuid,
    attempt_count: i32,
    order_id: Uuid,
    email_locale: String,
}

pub async fn enqueue_order_confirmation(
    transaction: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO notification_jobs (
            id, order_id, kind, deduplication_key, recipient_email
        )
        SELECT $2, id, 'order_confirmation', id::text, customer_email
        FROM orders WHERE id = $1
        ON CONFLICT (kind, deduplication_key) DO NOTHING
        "#,
    )
    .bind(order_id)
    .bind(Uuid::now_v7())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub async fn load_for_order(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Vec<NotificationStatus>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, kind, status, attempt_count, last_error,
               CASE WHEN sent_at IS NULL THEN NULL ELSE
                 to_char(sent_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') END AS sent_at,
               to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM notification_jobs WHERE order_id = $1 ORDER BY created_at, id
        "#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await
}

pub async fn deliver_due(
    pool: &PgPool,
    email: &EmailService,
    batch_size: i64,
) -> Result<DeliverySummary, sqlx::Error> {
    deliver_due_with_storage(pool, email, batch_size, None).await
}

pub async fn deliver_due_with_storage(
    pool: &PgPool,
    email: &EmailService,
    batch_size: i64,
    storage: Option<&ObjectStorage>,
) -> Result<DeliverySummary, sqlx::Error> {
    deliver_matching(pool, email, batch_size, storage, None).await
}

/// Attempt the first delivery after the payment transaction commits. Persistent
/// jobs let the worker recover failures, cancelled requests, and process exits.
pub async fn deliver_order_confirmation(
    pool: &PgPool,
    email: &EmailService,
    order_id: Uuid,
    storage: Option<&ObjectStorage>,
) {
    if let Err(error) = deliver_matching(pool, email, 1, storage, Some(order_id)).await {
        tracing::warn!(%error, %order_id, "immediate order email delivery failed; worker will recover the job");
    }
}

async fn deliver_matching(
    pool: &PgPool,
    email: &EmailService,
    batch_size: i64,
    storage: Option<&ObjectStorage>,
    order_id: Option<Uuid>,
) -> Result<DeliverySummary, sqlx::Error> {
    if !(1..=100).contains(&batch_size) {
        return Err(sqlx::Error::Protocol(
            "notification batch must be between 1 and 100".to_owned(),
        ));
    }
    let claimed = sqlx::query_as::<_, ClaimedNotification>(
        r#"
        WITH candidates AS (
            SELECT id FROM notification_jobs
            WHERE kind = 'order_confirmation' AND attempt_count < $1
              AND ($3::uuid IS NULL OR (order_id = $3 AND attempt_count = 0))
              AND (
                (status = 'pending' AND next_attempt_at <= now())
                OR (status = 'processing' AND locked_at <= now() - interval '15 minutes')
              )
            ORDER BY next_attempt_at, id
            FOR UPDATE SKIP LOCKED
            LIMIT $2
        ), claimed AS (
            UPDATE notification_jobs job
            SET status = 'processing', locked_at = now(),
                attempt_count = attempt_count + 1, updated_at = now()
            FROM candidates WHERE job.id = candidates.id
            RETURNING job.*
        )
        SELECT claimed.id, claimed.attempt_count, claimed.order_id, order_record.email_locale
        FROM claimed
        JOIN orders order_record ON order_record.id = claimed.order_id
        ORDER BY claimed.next_attempt_at, claimed.id
        "#,
    )
    .bind(MAX_ATTEMPTS)
    .bind(batch_size)
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    let mut summary = DeliverySummary::default();
    for job in claimed {
        let outcome = match load_order(pool, job.order_id).await {
            Ok(Some(order)) => {
                email
                    .send_confirmation(&order, &job.email_locale, pool, storage)
                    .await
            }
            Ok(None) => Err("order snapshot is unavailable".into()),
            Err(_) => Err("order snapshot could not be loaded".into()),
        };
        let mut transaction = pool.begin().await?;
        match outcome {
            Ok(()) => {
                sqlx::query(
                    "UPDATE notification_jobs SET status = 'sent', sent_at = now(), locked_at = NULL, last_error = NULL, updated_at = now() WHERE id = $1 AND status = 'processing' AND attempt_count = $2",
                )
                .bind(job.id)
                .bind(job.attempt_count)
                .execute(&mut *transaction)
                .await?;
                insert_attempt(&mut transaction, job.id, job.attempt_count, "sent", None).await?;
                summary.sent += 1;
            }
            Err(error) => {
                let safe_error = truncate_error(&error);
                let delay_seconds = retry_delay_seconds(job.attempt_count);
                let status = if job.attempt_count >= MAX_ATTEMPTS {
                    "failed"
                } else {
                    "pending"
                };
                sqlx::query(
                    r#"
                    UPDATE notification_jobs
                    SET status = $3, next_attempt_at = now() + make_interval(secs => $4),
                        locked_at = NULL, last_error = $5, updated_at = now()
                    WHERE id = $1 AND status = 'processing' AND attempt_count = $2
                    "#,
                )
                .bind(job.id)
                .bind(job.attempt_count)
                .bind(status)
                .bind(delay_seconds)
                .bind(&safe_error)
                .execute(&mut *transaction)
                .await?;
                insert_attempt(
                    &mut transaction,
                    job.id,
                    job.attempt_count,
                    "failed",
                    Some(&safe_error),
                )
                .await?;
                summary.failed += 1;
            }
        }
        transaction.commit().await?;
    }
    Ok(summary)
}

async fn insert_attempt(
    transaction: &mut Transaction<'_, Postgres>,
    job_id: Uuid,
    attempt_number: i32,
    outcome: &str,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO notification_attempts (
            id, notification_job_id, attempt_number, outcome, error_message
        ) VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (notification_job_id, attempt_number) DO NOTHING
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(job_id)
    .bind(attempt_number)
    .bind(outcome)
    .bind(error)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn retry_delay_seconds(attempt: i32) -> i64 {
    60_i64
        .saturating_mul(2_i64.saturating_pow(attempt.saturating_sub(1) as u32))
        .min(21_600)
}

fn truncate_error(error: &str) -> String {
    error.chars().take(1000).collect()
}

#[cfg(test)]
mod tests {
    use super::retry_delay_seconds;

    #[test]
    fn notification_retries_are_bounded() {
        assert_eq!(retry_delay_seconds(1), 60);
        assert_eq!(retry_delay_seconds(2), 120);
        assert_eq!(retry_delay_seconds(20), 21_600);
    }
}
