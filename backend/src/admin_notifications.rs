use axum::{
    Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
    notifications::{ClaimedNotification, deliver_claimed},
};

#[derive(Deserialize, IntoParams)]
pub struct NotificationQuery {
    /// Case-insensitive search across recipient, order number, job ID, and kind.
    pub q: Option<String>,
    /// outstanding (default), pending, processing, failed, sent, suppressed, or all.
    pub status: Option<String>,
    pub kind: Option<String>,
    /// One-based page number.
    pub page: Option<u32>,
    /// Between 1 and 100; defaults to 25.
    pub page_size: Option<u32>,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct AdminNotification {
    pub id: Uuid,
    pub order_id: Option<Uuid>,
    pub order_number: Option<String>,
    pub recipient_email: String,
    pub kind: String,
    pub status: String,
    pub attempt_count: i32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub next_attempt_at: String,
    pub sent_at: Option<String>,
    /// False for unsupported kinds, completed jobs, or active delivery claims.
    pub can_send: bool,
}

#[derive(Serialize, ToSchema)]
pub struct NotificationPage {
    pub items: Vec<AdminNotification>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Deserialize, ToSchema)]
pub struct SendNotificationRequest {
    pub reason: String,
}

const FIELDS: &str = r#"
    job.id, job.order_id, order_record.order_number, job.recipient_email,
    job.kind, job.status, job.attempt_count, job.last_error,
    to_char(job.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
    to_char(job.next_attempt_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS next_attempt_at,
    to_char(job.sent_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS sent_at,
    (job.kind = 'order_confirmation' AND job.order_id IS NOT NULL AND (
        job.status IN ('pending', 'failed') OR
        (job.status = 'processing' AND job.locked_at <= now() - interval '15 minutes')
    )) AS can_send
"#;

const FILTER: &str = r#"
    FROM notification_jobs job
    LEFT JOIN orders order_record ON order_record.id = job.order_id
    WHERE ($1 = 'all' OR job.status = $1 OR
        ($1 = 'outstanding' AND job.status IN ('pending', 'processing', 'failed')))
      AND ($2 = '' OR strpos(lower(job.recipient_email), lower($2)) > 0
        OR strpos(lower(COALESCE(order_record.order_number, '')), lower($2)) > 0
        OR strpos(job.id::text, lower($2)) > 0
        OR strpos(lower(job.kind), lower($2)) > 0)
      AND ($3 = '' OR job.kind = $3)
"#;

#[utoipa::path(
    get, path = "/api/admin/notifications", tag = "admin notifications",
    params(NotificationQuery),
    responses(
        (status = 200, body = NotificationPage),
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn list(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Query(query): Query<NotificationQuery>,
) -> Response {
    if let Err(response) = require_capability(&actor, "notifications.read") {
        return response.into_response();
    }
    let status = query.status.as_deref().unwrap_or("outstanding");
    let q = query.q.as_deref().unwrap_or("").trim();
    let kind = query.kind.as_deref().unwrap_or("").trim();
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(25);
    if ![
        "outstanding",
        "all",
        "pending",
        "processing",
        "failed",
        "sent",
        "suppressed",
    ]
    .contains(&status)
        || q.len() > 320
        || kind.len() > 100
        || !(1..=100_000).contains(&page)
        || !(1..=100).contains(&page_size)
    {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_notification_query",
            "Provide valid notification filters and pagination.",
        );
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let total = sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) {FILTER}"))
        .bind(status)
        .bind(q)
        .bind(kind)
        .fetch_one(&pool)
        .await;
    let items = sqlx::query_as::<_, AdminNotification>(&format!(
        "SELECT {FIELDS} {FILTER} ORDER BY job.created_at DESC, job.id DESC LIMIT $4 OFFSET $5"
    ))
    .bind(status)
    .bind(q)
    .bind(kind)
    .bind(i64::from(page_size))
    .bind(i64::from(page - 1) * i64::from(page_size))
    .fetch_all(&pool)
    .await;
    match (total, items) {
        (Ok(total), Ok(items)) => no_store(
            Json(NotificationPage {
                items,
                total,
                page,
                page_size,
            })
            .into_response(),
        ),
        _ => unavailable(),
    }
}

#[utoipa::path(
    post, path = "/api/admin/notifications/{notification_id}/send", tag = "admin notifications",
    params(("notification_id" = Uuid, Path)), request_body = SendNotificationRequest,
    responses(
        (status = 200, description = "Delivery attempted; inspect status and last_error for the outcome", body = AdminNotification),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, description = "Already sent, suppressed, actively processing, or unsupported kind", body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn send(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(notification_id): Path<Uuid>,
    Json(input): Json<SendNotificationRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, "notifications.send") {
        return response.into_response();
    }
    let reason = input.reason.trim();
    if !(3..=500).contains(&reason.chars().count()) {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_notification_reason",
            "Provide a reason of 3–500 characters.",
        );
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let result = claim_manual(&pool, notification_id, actor.id, reason).await;
    match result {
        Ok(Some(job)) => {
            if let Err(error) =
                deliver_claimed(&pool, &state.email, state.media_storage.as_ref(), vec![job]).await
            {
                tracing::warn!(%error, %notification_id, "manual notification delivery interrupted");
                return unavailable();
            }
            match load(&pool, notification_id).await {
                Ok(Some(job)) => no_store(Json(job).into_response()),
                _ => unavailable(),
            }
        }
        Ok(None) => match load(&pool, notification_id).await {
            Ok(Some(_)) => error(
                StatusCode::CONFLICT,
                "notification_not_sendable",
                "This notification is already sent, suppressed, actively processing, or has no delivery handler.",
            ),
            Ok(None) => error(
                StatusCode::NOT_FOUND,
                "notification_not_found",
                "The notification was not found.",
            ),
            Err(_) => unavailable(),
        },
        Err(_) => unavailable(),
    }
}

async fn load(pool: &PgPool, id: Uuid) -> Result<Option<AdminNotification>, sqlx::Error> {
    sqlx::query_as(&format!("SELECT {FIELDS} FROM notification_jobs job LEFT JOIN orders order_record ON order_record.id = job.order_id WHERE job.id = $1"))
        .bind(id).fetch_optional(pool).await
}

async fn claim_manual(
    pool: &PgPool,
    id: Uuid,
    actor_id: Uuid,
    reason: &str,
) -> Result<Option<ClaimedNotification>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    // Claim and audit together, before contacting the email provider. The same
    // processing lease is respected by the API, worker, and other admin sends.
    let job = sqlx::query_as::<_, ClaimedNotification>(
        r#"
        WITH claimed AS (
            UPDATE notification_jobs SET status = 'processing', locked_at = now(),
                attempt_count = attempt_count + 1, updated_at = now()
            WHERE id = $1 AND kind = 'order_confirmation' AND order_id IS NOT NULL
              AND (status IN ('pending', 'failed') OR
                  (status = 'processing' AND locked_at <= now() - interval '15 minutes'))
            RETURNING id, attempt_count, order_id
        )
        SELECT claimed.id, claimed.attempt_count, claimed.order_id, order_record.email_locale
        FROM claimed JOIN orders order_record ON order_record.id = claimed.order_id
    "#,
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    if job.is_some() {
        sqlx::query("INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason) VALUES ($1, 'notification.manual_send', 'notification_job', $2, $3)")
            .bind(actor_id).bind(id.to_string()).bind(reason).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(job)
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store, private".parse().unwrap());
    response
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    no_store((status, Json(ErrorBody::new(code, message))).into_response())
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "notifications_unavailable",
        "Notifications are temporarily unavailable.",
    )
}
