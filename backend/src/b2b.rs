use std::io::Cursor;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    customers::valid_email,
    email::EmailService,
    error::ErrorBody,
    login_rate_limit::{ClientIp, LoginLimitError, consume_account_action},
    media_scanner::ScanOutcome,
    notifications::DeliverySummary,
};

pub const MAX_ATTACHMENT_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_REQUEST_BYTES: usize = 14 * 1024 * 1024;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct CreateB2bRequest {
    pub request_key: Uuid,
    pub company: String,
    pub contact_name: String,
    pub email: String,
    pub phone: String,
    pub product_type: String,
    pub quantity: i32,
    pub notes: String,
    pub attachment_name: String,
    /// Original file encoded as base64, at most 10 MiB decoded.
    pub attachment_base64: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct B2bReceipt {
    pub id: Uuid,
    pub reference: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct B2bRequest {
    pub id: Uuid,
    pub reference: String,
    pub company: String,
    pub contact_name: String,
    pub email: String,
    pub phone: String,
    pub product_type: String,
    pub quantity: i32,
    pub notes: String,
    pub status: String,
    pub attachment_name: String,
    pub attachment_type: String,
    pub attachment_size: i32,
    pub has_preview: bool,
    pub notification_status: String,
    pub notification_attempts: i32,
    pub notification_sent_at: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub offset: Option<u32>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateB2bStatus {
    pub status: String,
}

const REQUEST_SELECT: &str = r#"
    SELECT r.id, 'B2B-' || r.reference_number AS reference, r.company, r.contact_name,
        r.email, r.phone, r.product_type, r.quantity, r.notes, r.status,
        r.attachment_name, r.attachment_type, octet_length(r.attachment_bytes) AS attachment_size,
        r.preview_bytes IS NOT NULL AS has_preview,
        j.status AS notification_status, j.attempt_count AS notification_attempts,
        to_char(j.sent_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS notification_sent_at,
        to_char(r.created_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
    FROM b2b_requests r JOIN b2b_notification_jobs j ON j.request_id=r.id
"#;

#[utoipa::path(post, path = "/api/b2b/requests", tag = "B2B", request_body = CreateB2bRequest,
    responses((status = 201, body = B2bReceipt), (status = 200, body = B2bReceipt),
        (status = 409, body = ErrorBody), (status = 422, body = ErrorBody),
        (status = 429, body = ErrorBody), (status = 503, body = ErrorBody)))]
pub async fn create(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(mut input): Json<CreateB2bRequest>,
) -> Response {
    input.company = input.company.trim().to_owned();
    input.contact_name = input.contact_name.trim().to_owned();
    input.email = input.email.trim().to_lowercase();
    input.phone = input.phone.trim().to_owned();
    input.notes = input.notes.trim().to_owned();
    if !valid_input(&input) {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_b2b_request",
            "Check the contact details, quantity and attachment (maximum 10 MB).",
        );
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let hash = Sha256::digest(serde_json::to_vec(&input).expect("serializable request")).to_vec();
    match existing(&pool, input.request_key, &hash).await {
        Ok(Some(receipt)) => return Json(receipt).into_response(),
        Ok(None) => {}
        Err(response) => return response,
    }
    match consume_account_action(&pool, "b2b", &input.email, ip).await {
        Ok(()) => {}
        Err(LoginLimitError::Limited(seconds)) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, seconds.to_string())],
                Json(ErrorBody::new(
                    "b2b_rate_limited",
                    "Too many requests. Please try again later.",
                )),
            )
                .into_response();
        }
        Err(_) => return unavailable(),
    }
    let bytes = match STANDARD.decode(&input.attachment_base64) {
        Ok(bytes) if !bytes.is_empty() && bytes.len() <= MAX_ATTACHMENT_BYTES => bytes,
        _ => return invalid_attachment(),
    };
    let attachment_type = match attachment_type(&input.attachment_name, &bytes) {
        Some(kind) => kind,
        None => return invalid_attachment(),
    };
    match state.media_scanner.scan(&bytes).await {
        Ok(ScanOutcome::Clean) => {}
        Ok(ScanOutcome::Infected(_)) => return invalid_attachment(),
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "b2b_scan_unavailable",
                "The attachment could not be checked. Please try again.",
            );
        }
    }
    let preview = if matches!(attachment_type, "image/png" | "image/jpeg") {
        let source = bytes.clone();
        match tokio::task::spawn_blocking(move || bitmap_preview(&source)).await {
            Ok(Ok(preview)) => Some((preview, "image/png")),
            _ => return invalid_attachment(),
        }
    } else if attachment_type == "image/svg+xml" {
        // SVG is served only as an <img>, with a sandboxed CSP, never embedded as HTML.
        Some((bytes.clone(), "image/svg+xml"))
    } else {
        None
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let id = Uuid::now_v7();
    let inserted: Option<B2bReceipt> = match sqlx::query_as(
        r#"INSERT INTO b2b_requests (id,request_key,request_hash,company,contact_name,email,phone,
            product_type,quantity,notes,attachment_name,attachment_type,attachment_bytes,
            attachment_sha256,preview_bytes,preview_type)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
        ON CONFLICT (request_key) DO NOTHING
        RETURNING id, 'B2B-' || reference_number AS reference"#,
    )
    .bind(id)
    .bind(input.request_key)
    .bind(&hash)
    .bind(&input.company)
    .bind(&input.contact_name)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.product_type)
    .bind(input.quantity)
    .bind(&input.notes)
    .bind(&input.attachment_name)
    .bind(attachment_type)
    .bind(&bytes)
    .bind(Sha256::digest(&bytes).to_vec())
    .bind(preview.as_ref().map(|p| &p.0))
    .bind(preview.as_ref().map(|p| p.1))
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(receipt) => receipt,
        Err(_) => return unavailable(),
    };
    let Some(receipt) = inserted else {
        let _ = tx.rollback().await;
        return match existing(&pool, input.request_key, &hash).await {
            Ok(Some(receipt)) => Json(receipt).into_response(),
            Err(response) => response,
            _ => unavailable(),
        };
    };
    if sqlx::query("INSERT INTO b2b_notification_jobs (request_id) VALUES ($1)")
        .bind(id)
        .execute(&mut *tx)
        .await
        .is_err()
        || tx.commit().await.is_err()
    {
        return unavailable();
    }
    (StatusCode::CREATED, Json(receipt)).into_response()
}

async fn existing(pool: &PgPool, key: Uuid, hash: &[u8]) -> Result<Option<B2bReceipt>, Response> {
    let row: Option<(Uuid, String, Vec<u8>)> = sqlx::query_as(
        "SELECT id, 'B2B-' || reference_number, request_hash FROM b2b_requests WHERE request_key=$1",
    ).bind(key).fetch_optional(pool).await.map_err(|_| unavailable())?;
    match row {
        Some((id, reference, stored)) if stored == hash => Ok(Some(B2bReceipt { id, reference })),
        Some(_) => Err(error(
            StatusCode::CONFLICT,
            "b2b_request_conflict",
            "This request has already been submitted with different details.",
        )),
        None => Ok(None),
    }
}

#[utoipa::path(get, path = "/api/admin/b2b/requests", tag = "admin B2B",
    responses((status = 200, body = [B2bRequest]), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody)))]
pub async fn list(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Query(query): Query<ListQuery>,
) -> Response {
    if let Err(response) = require_capability(&actor, "orders.read") {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match sqlx::query_as::<_, B2bRequest>(&format!(
        "{REQUEST_SELECT} ORDER BY r.created_at DESC,r.id DESC LIMIT 26 OFFSET $1"
    ))
    .bind(i64::from(query.offset.unwrap_or(0).min(1_000_000)))
    .fetch_all(&pool)
    .await
    {
        Ok(requests) => private_json(requests),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(get, path = "/api/admin/b2b/requests/{id}", tag = "admin B2B", params(("id" = Uuid, Path)),
    responses((status = 200, body = B2bRequest), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub async fn detail(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_capability(&actor, "orders.read") {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match sqlx::query_as::<_, B2bRequest>(&format!("{REQUEST_SELECT} WHERE r.id=$1"))
        .bind(id)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(request)) => private_json(request),
        Ok(None) => not_found(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(put, path = "/api/admin/b2b/requests/{id}/status", tag = "admin B2B", params(("id" = Uuid, Path)),
    request_body = UpdateB2bStatus, responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 422, body = ErrorBody)))]
pub async fn update_status(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateB2bStatus>,
) -> Response {
    if let Err(response) = require_capability(&actor, "orders.fulfill") {
        return response.into_response();
    }
    if !["new", "in_progress", "completed"].contains(&input.status.as_str()) {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_b2b_status",
            "Choose a valid request status.",
        );
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    match sqlx::query("UPDATE b2b_requests SET status=$2,updated_at=now() WHERE id=$1")
        .bind(id)
        .bind(&input.status)
        .execute(&mut *tx)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => return not_found(),
        Err(_) => return unavailable(),
        _ => {}
    }
    if sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,metadata) VALUES ($1,'b2b.status_update','b2b_request',$2,jsonb_build_object('status',$3::text))")
        .bind(actor.id).bind(id.to_string()).bind(input.status).execute(&mut *tx).await.is_err() || tx.commit().await.is_err() {
        return unavailable();
    }
    StatusCode::NO_CONTENT.into_response()
}

#[utoipa::path(delete, path = "/api/admin/b2b/requests/{id}", tag = "admin B2B", params(("id" = Uuid, Path)),
    responses((status = 204), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody), (status = 409, body = ErrorBody), (status = 503, body = ErrorBody)))]
pub async fn delete(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_capability(&actor, "orders.fulfill") {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    // Serialize with notification claims so deletion cannot race an active email send.
    let status: Option<String> = match sqlx::query_scalar(
        "SELECT status FROM b2b_notification_jobs WHERE request_id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(status) => status,
        Err(_) => return unavailable(),
    };
    match status.as_deref() {
        None => return not_found(),
        Some("processing") => {
            return error(
                StatusCode::CONFLICT,
                "b2b_notification_in_progress",
                "The support notification is being sent. Please try deleting the request again in a moment.",
            );
        }
        _ => {}
    }
    if sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,metadata)
        SELECT $1,'b2b.delete','b2b_request',id::text,jsonb_build_object('reference','B2B-' || reference_number)
        FROM b2b_requests WHERE id=$2")
        .bind(actor.id).bind(id).execute(&mut *tx).await.is_err() {
        return unavailable();
    }
    if sqlx::query("DELETE FROM b2b_notification_jobs WHERE request_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .is_err()
    {
        return unavailable();
    }
    // Originals and previews are bytea columns of the same row and are removed atomically.
    match sqlx::query("DELETE FROM b2b_requests WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => return not_found(),
        Err(_) => return unavailable(),
        _ => {}
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
pub struct AttachmentQuery {
    pub preview: Option<bool>,
}

#[utoipa::path(get, path = "/api/admin/b2b/requests/{id}/attachment", tag = "admin B2B", params(("id" = Uuid, Path)),
    responses((status = 200, description = "Private original attachment or image preview"), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub async fn attachment(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(id): Path<Uuid>,
    Query(query): Query<AttachmentQuery>,
) -> Response {
    if let Err(response) = require_capability(&actor, "orders.read") {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let preview = query.preview.unwrap_or(false);
    let row: Option<(String, Option<String>, Option<Vec<u8>>)> = match sqlx::query_as(
        "SELECT attachment_name, CASE WHEN $2 THEN preview_type ELSE attachment_type END,
            CASE WHEN $2 THEN preview_bytes ELSE attachment_bytes END FROM b2b_requests WHERE id=$1",
    ).bind(id).bind(preview).fetch_optional(&pool).await {
        Ok(row) => row,
        Err(_) => return unavailable(),
    };
    let Some((name, Some(kind), Some(bytes))) = row else {
        return not_found();
    };
    let filename: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&kind)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "{}; filename=\"{filename}\"",
            if preview { "inline" } else { "attachment" }
        ))
        .expect("safe filename"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response
}

fn private_json(value: impl Serialize) -> Response {
    ([(header::CACHE_CONTROL, "private, no-store")], Json(value)).into_response()
}

fn valid_input(input: &CreateB2bRequest) -> bool {
    fn bounded(value: &str, min: usize, max: usize) -> bool {
        (min..=max).contains(&value.chars().count()) && !value.chars().any(char::is_control)
    }
    !input.request_key.is_nil()
        && bounded(&input.company, 1, 200)
        && bounded(&input.contact_name, 1, 150)
        && valid_email(&input.email)
        && input.email.len() <= 254
        && bounded(&input.phone, 3, 40)
        && (1..=1_000_000).contains(&input.quantity)
        && ["clothing", "bottles", "backpacks", "complete-kit", "other"]
            .contains(&input.product_type.as_str())
        && input.notes.chars().count() <= 5000
        && !input.notes.contains('\0')
        && bounded(&input.attachment_name, 1, 200)
        && !input.attachment_name.contains(['/', '\\'])
        && input.attachment_base64.len() <= MAX_ATTACHMENT_BYTES.div_ceil(3) * 4
}

fn attachment_type(name: &str, bytes: &[u8]) -> Option<&'static str> {
    match name.rsplit('.').next()?.to_ascii_lowercase().as_str() {
        "png" if bytes.starts_with(b"\x89PNG\r\n\x1a\n") => Some("image/png"),
        "jpg" | "jpeg" if bytes.starts_with(b"\xff\xd8\xff") => Some("image/jpeg"),
        "pdf" | "ai" if bytes.starts_with(b"%PDF-") => Some("application/pdf"),
        "eps" | "ai" if bytes.starts_with(b"%!PS-Adobe-") => Some("application/postscript"),
        "svg" => {
            let text = std::str::from_utf8(bytes)
                .ok()?
                .trim_start_matches('\u{feff}')
                .trim();
            if (text.starts_with("<svg") || text.starts_with("<?xml"))
                && text.contains("<svg")
                && !text.contains("<!ENTITY")
            {
                Some("image/svg+xml")
            } else {
                None
            }
        }
        _ => None,
    }
}

fn bitmap_preview(bytes: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(12000);
    limits.max_image_height = Some(12000);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?.thumbnail(1600, 1600);
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, image::ImageFormat::Png)?;
    Ok(output.into_inner())
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}
fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "b2b_unavailable",
        "Business requests are temporarily unavailable. Please try again.",
    )
}
fn invalid_attachment() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_b2b_attachment",
        "Choose a valid AI, EPS, PDF, SVG, PNG or JPG file, up to 10 MB.",
    )
}
fn not_found() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "b2b_not_found",
        "Business request or attachment not found.",
    )
}

/// Durable outbox: claims are exclusive; interrupted deliveries are recovered after 15 minutes.
pub async fn deliver_due(
    pool: &PgPool,
    email: &EmailService,
    batch: i64,
) -> Result<DeliverySummary, sqlx::Error> {
    sqlx::query("UPDATE b2b_notification_jobs SET status='failed',locked_at=NULL,
        last_error=COALESCE(last_error,'Delivery interrupted during the final attempt')
        WHERE status='processing' AND attempt_count >= 8 AND locked_at <= now()-interval '15 minutes'")
        .execute(pool).await?;
    #[derive(FromRow)]
    struct Job {
        request_id: Uuid,
        attempt_count: i32,
    }
    let jobs = sqlx::query_as::<_, Job>(r#"
        WITH candidates AS (
            SELECT request_id FROM b2b_notification_jobs
            WHERE attempt_count < 8 AND ((status='pending' AND next_attempt_at <= now())
                OR (status='processing' AND locked_at <= now()-interval '15 minutes'))
            ORDER BY next_attempt_at,request_id FOR UPDATE SKIP LOCKED LIMIT $1
        ) UPDATE b2b_notification_jobs j SET status='processing',locked_at=now(),attempt_count=attempt_count+1
        FROM candidates c WHERE j.request_id=c.request_id RETURNING j.request_id,j.attempt_count
    "#).bind(batch.clamp(1,100)).fetch_all(pool).await?;
    let mut summary = DeliverySummary::default();
    for job in jobs {
        let request = sqlx::query_as::<_, B2bRequest>(&format!("{REQUEST_SELECT} WHERE r.id=$1"))
            .bind(job.request_id)
            .fetch_one(pool)
            .await?;
        let result = email.send_b2b_notification(&request).await;
        let (status, last_error) = match result {
            Ok(()) => {
                summary.sent += 1;
                ("sent", None)
            }
            Err(message) => {
                summary.failed += 1;
                (
                    if job.attempt_count >= 8 {
                        "failed"
                    } else {
                        "pending"
                    },
                    Some(message.chars().take(1000).collect::<String>()),
                )
            }
        };
        let delay = 60_i64 * 2_i64.pow(job.attempt_count.saturating_sub(1).min(8) as u32);
        sqlx::query("UPDATE b2b_notification_jobs SET status=$3,last_error=$4,locked_at=NULL,
            sent_at=CASE WHEN $3='sent' THEN now() ELSE NULL END,next_attempt_at=now()+make_interval(secs=>$5)
            WHERE request_id=$1 AND attempt_count=$2 AND status='processing'")
            .bind(job.request_id).bind(job.attempt_count).bind(status).bind(last_error).bind(delay)
            .execute(pool).await?;
    }
    Ok(summary)
}
