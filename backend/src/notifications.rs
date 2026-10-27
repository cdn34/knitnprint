use serde::Serialize;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::email::{EmailService, OrderEmail, OrderEmailKind};

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
    recipient_email: String,
    kind: String,
    order_id: Uuid,
    locale: String,
    order_number: String,
    customer_first_name: String,
    total_minor: i64,
    currency: String,
    carrier: String,
    tracking_number: String,
    tracking_url: String,
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

pub async fn enqueue_fulfillment(
    transaction: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    fulfillment_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO notification_jobs (
            id, order_id, fulfillment_id, kind, deduplication_key, recipient_email
        )
        SELECT $3, id, $2, 'fulfillment_created', $2::text, customer_email
        FROM orders WHERE id = $1
        ON CONFLICT (kind, deduplication_key) DO NOTHING
        "#,
    )
    .bind(order_id)
    .bind(fulfillment_id)
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
    deliver_due_with_storage(pool, email, None, batch_size).await
}

pub async fn deliver_due_with_storage(
    pool: &PgPool,
    email: &EmailService,
    storage: Option<&crate::object_storage::ObjectStorage>,
    batch_size: i64,
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
            WHERE attempt_count < $1
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
        SELECT claimed.id, claimed.attempt_count, claimed.recipient_email, claimed.kind,
               claimed.order_id, order_record.locale,
               order_record.order_number, order_record.customer_first_name,
               order_record.total_minor, order_record.currency::text AS currency,
               COALESCE(fulfillment.carrier, '') AS carrier,
               COALESCE(fulfillment.tracking_number, '') AS tracking_number,
               COALESCE(fulfillment.tracking_url, '') AS tracking_url
        FROM claimed
        JOIN orders order_record ON order_record.id = claimed.order_id
        LEFT JOIN fulfillments fulfillment ON fulfillment.id = claimed.fulfillment_id
        ORDER BY claimed.next_attempt_at, claimed.id
        "#,
    )
    .bind(MAX_ATTEMPTS)
    .bind(batch_size)
    .fetch_all(pool)
    .await?;

    let mut summary = DeliverySummary::default();
    for job in claimed {
        let kind = if job.kind == "order_confirmation" {
            OrderEmailKind::Confirmation
        } else {
            OrderEmailKind::Fulfillment
        };
        let total = format_money(job.total_minor, &job.currency);
        let outcome = if kind == OrderEmailKind::Confirmation {
            deliver_confirmation(pool, email, storage, &job).await
        } else {
            email
                .send_order_notification(OrderEmail {
                    to: &job.recipient_email,
                    first_name: &job.customer_first_name,
                    kind,
                    order_number: &job.order_number,
                    total: &total,
                    carrier: &job.carrier,
                    tracking_number: &job.tracking_number,
                    tracking_url: &job.tracking_url,
                })
                .await
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
        INSERT INTO notification_attempts (id, notification_job_id, attempt_number, outcome, error_message)
        VALUES ($1, $2, $3, $4, $5) ON CONFLICT (notification_job_id, attempt_number) DO NOTHING
        "#,
    ).bind(Uuid::now_v7()).bind(job_id).bind(attempt_number).bind(outcome).bind(error)
        .execute(&mut **transaction).await?;
    Ok(())
}

async fn deliver_confirmation(
    pool: &PgPool,
    email: &EmailService,
    storage: Option<&crate::object_storage::ObjectStorage>,
    job: &ClaimedNotification,
) -> Result<(), String> {
    let mut order = crate::orders::load_order(pool, job.order_id)
        .await
        .map_err(|_| "could not load confirmation snapshot")?
        .ok_or("order snapshot missing")?;
    // Use the recipient captured in the outbox, not the mutable account address.
    order.customer.email.clone_from(&job.recipient_email);
    let mut images = Vec::new();
    let mut total_bytes = 0_usize;
    for line in &order.lines {
        if line.customization.is_none() {
            if let Some(id) = line
                .personalization_context
                .as_ref()
                .and_then(|context| context.pointer("/views/0/media_id"))
                .and_then(|v| v.as_str())
                .and_then(|v| Uuid::parse_str(v).ok())
            {
                let key: String = sqlx::query_scalar("SELECT variant.object_key FROM media_variants variant JOIN media_assets media ON media.id=variant.media_asset_id WHERE media.id=$1 AND media.status='ready' AND variant.kind='card'")
                    .bind(id).fetch_one(pool).await.map_err(|_| "order product image unavailable")?;
                let object = storage
                    .ok_or("object storage is required for confirmation images")?
                    .get(&key)
                    .await
                    .map_err(|_| "could not read order product image")?;
                let mut png = std::io::Cursor::new(Vec::new());
                image::load_from_memory(&object.bytes)
                    .and_then(|image| {
                        image
                            .thumbnail(900, 900)
                            .write_to(&mut png, image::ImageFormat::Png)
                    })
                    .map_err(|_| "could not prepare order product image")?;
                total_bytes = total_bytes.saturating_add(png.get_ref().len());
                if total_bytes > 20_000_000 || images.len() >= 40 {
                    return Err("confirmation image attachments exceed delivery limit".into());
                }
                images.push(crate::order_email::InlineImage {
                    cid: format!("catalog-{}", line.id),
                    bytes: png.into_inner(),
                    content_type: "image/png".into(),
                });
            }
            continue;
        }
        let Some(previews) = line
            .customization
            .as_ref()
            .and_then(|v| v.get("previews"))
            .and_then(|v| v.as_array())
        else {
            continue;
        };
        for preview in previews {
            let id = preview
                .get("media_id")
                .and_then(|v| v.as_str())
                .and_then(|v| Uuid::parse_str(v).ok())
                .ok_or("invalid saved preview")?;
            let cid = format!("preview-{id}");
            if images
                .iter()
                .any(|image: &crate::order_email::InlineImage| image.cid == cid)
            {
                continue;
            }
            let key: String = sqlx::query_scalar("SELECT media.object_key FROM media_assets media JOIN order_line_personalization_media retained ON retained.media_asset_id=media.id WHERE retained.order_line_id=$1 AND media.id=$2 AND media.status='ready'")
                .bind(line.id).bind(id).fetch_one(pool).await.map_err(|_| "saved preview unavailable")?;
            let storage =
                storage.ok_or("object storage is required for personalized confirmation images")?;
            let object = storage
                .get(&key)
                .await
                .map_err(|_| "could not read saved preview")?;
            total_bytes = total_bytes.saturating_add(object.bytes.len());
            if total_bytes > 20_000_000 || images.len() >= 40 {
                return Err("confirmation preview attachments exceed delivery limit".into());
            }
            images.push(crate::order_email::InlineImage {
                cid,
                bytes: object.bytes,
                content_type: "image/png".into(),
            });
        }
    }
    email
        .send_confirmation(
            &order,
            crate::order_email::EmailLocale::from_code(Some(&job.locale)),
            images,
        )
        .await
}

fn retry_delay_seconds(attempt: i32) -> i64 {
    60_i64
        .saturating_mul(2_i64.saturating_pow(attempt.saturating_sub(1) as u32))
        .min(21_600)
}

fn truncate_error(error: &str) -> String {
    error.chars().take(1000).collect()
}

fn format_money(minor: i64, currency: &str) -> String {
    format!("{} {:.2}", currency, minor as f64 / 100.0)
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
