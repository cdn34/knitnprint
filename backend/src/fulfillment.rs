use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
    notifications::enqueue_fulfillment,
    orders::{Order, load_order},
};

const ORDERS_FULFILL: &str = "orders.fulfill";

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateFulfillmentRequest {
    pub carrier: String,
    pub tracking_number: String,
    pub tracking_url: String,
    pub reason: String,
    pub lines: Vec<CreateFulfillmentLineRequest>,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateFulfillmentLineRequest {
    pub order_line_id: Uuid,
    pub quantity: i32,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct Fulfillment {
    pub id: Uuid,
    pub carrier: String,
    pub tracking_number: String,
    pub tracking_url: String,
    pub reason: String,
    pub actor_display_name: Option<String>,
    pub lines: Vec<FulfillmentLine>,
    pub created_at: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct FulfillmentLine {
    pub order_line_id: Uuid,
    pub product_title: String,
    pub variant_title: String,
    pub sku: String,
    pub quantity: i32,
}

#[derive(FromRow)]
struct FulfillmentHead {
    id: Uuid,
    carrier: String,
    tracking_number: String,
    tracking_url: String,
    reason: String,
    actor_display_name: Option<String>,
    created_at: String,
}

#[derive(FromRow)]
struct LockedOrder {
    order_status: String,
    payment_status: String,
}

#[derive(FromRow)]
struct LineAvailability {
    id: Uuid,
    quantity: i32,
    fulfilled_quantity: i64,
}

#[utoipa::path(
    post,
    path = "/api/admin/orders/{order_id}/fulfillments",
    tag = "admin orders",
    params(("order_id" = Uuid, Path), ("Idempotency-Key" = String, Header)),
    request_body = CreateFulfillmentRequest,
    responses(
        (status = 200, body = Order, description = "Idempotent replay"),
        (status = 201, body = Order),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CreateFulfillmentRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, ORDERS_FULFILL) {
        return response.into_response();
    }
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return fulfillment_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_idempotency_key",
            "Provide a 16–128 character Idempotency-Key for fulfillment.",
        );
    };
    let Ok(validated) = ValidatedFulfillment::new(input) else {
        return fulfillment_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_fulfillment",
            "Provide valid quantities, a reason, and optional HTTPS tracking details.",
        );
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    match create_fulfillment(&pool, order_id, actor.id, idempotency_hash, &validated).await {
        Ok(created) => match load_order(&pool, order_id).await {
            Ok(Some(order)) => (
                if created {
                    StatusCode::CREATED
                } else {
                    StatusCode::OK
                },
                Json(order),
            )
                .into_response(),
            _ => unavailable(),
        },
        Err(CreateError::NotFound) => fulfillment_error(
            StatusCode::NOT_FOUND,
            "order_not_found",
            "The order was not found.",
        ),
        Err(CreateError::NotPaid) => fulfillment_error(
            StatusCode::CONFLICT,
            "order_not_fulfillable",
            "Only paid, uncancelled orders can be fulfilled.",
        ),
        Err(CreateError::QuantityConflict) => fulfillment_error(
            StatusCode::CONFLICT,
            "fulfillment_quantity_conflict",
            "A fulfillment quantity exceeds the remaining order quantity.",
        ),
        Err(CreateError::IdempotencyConflict) => fulfillment_error(
            StatusCode::CONFLICT,
            "fulfillment_idempotency_conflict",
            "That fulfillment key was already used for a different request.",
        ),
        Err(CreateError::Database) => unavailable(),
    }
}

struct ValidatedFulfillment {
    carrier: String,
    tracking_number: String,
    tracking_url: String,
    reason: String,
    lines: Vec<CreateFulfillmentLineRequest>,
    request_hash: [u8; 32],
}

impl ValidatedFulfillment {
    fn new(input: CreateFulfillmentRequest) -> Result<Self, ()> {
        let carrier = input.carrier.trim().to_owned();
        let tracking_number = input.tracking_number.trim().to_owned();
        let tracking_url = input.tracking_url.trim().to_owned();
        let reason = input.reason.trim().to_owned();
        let mut line_ids = HashSet::new();
        if input.lines.is_empty()
            || input.lines.len() > 100
            || input.lines.iter().any(|line| {
                !(1..=99).contains(&line.quantity) || !line_ids.insert(line.order_line_id)
            })
            || carrier.len() > 100
            || tracking_number.len() > 200
            || tracking_url.len() > 2000
            || (!tracking_url.is_empty() || !tracking_number.is_empty()) && carrier.is_empty()
            || (!tracking_url.is_empty() && !tracking_url.starts_with("https://"))
            || !(3..=500).contains(&reason.len())
        {
            return Err(());
        }
        let mut lines = input.lines;
        lines.sort_by_key(|line| line.order_line_id);
        let signature =
            serde_json::to_vec(&(&carrier, &tracking_number, &tracking_url, &reason, &lines))
                .map_err(|_| ())?;
        Ok(Self {
            carrier,
            tracking_number,
            tracking_url,
            reason,
            lines,
            request_hash: Sha256::digest(signature).into(),
        })
    }
}

enum CreateError {
    NotFound,
    NotPaid,
    QuantityConflict,
    IdempotencyConflict,
    Database,
}

async fn create_fulfillment(
    pool: &PgPool,
    order_id: Uuid,
    actor_id: Uuid,
    idempotency_hash: [u8; 32],
    input: &ValidatedFulfillment,
) -> Result<bool, CreateError> {
    let mut transaction = pool.begin().await.map_err(|_| CreateError::Database)?;
    let order = sqlx::query_as::<_, LockedOrder>(
        r#"
        SELECT order_status, payment_status
        FROM orders WHERE id = $1 FOR UPDATE
        "#,
    )
    .bind(order_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?
    .ok_or(CreateError::NotFound)?;

    let existing: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT request_hash FROM fulfillments WHERE order_id = $1 AND idempotency_hash = $2",
    )
    .bind(order_id)
    .bind(idempotency_hash.as_slice())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    if let Some(existing_hash) = existing {
        if existing_hash.as_slice() != input.request_hash {
            return Err(CreateError::IdempotencyConflict);
        }
        transaction
            .commit()
            .await
            .map_err(|_| CreateError::Database)?;
        return Ok(false);
    }
    if order.payment_status != "paid" || !matches!(order.order_status.as_str(), "confirmed") {
        return Err(CreateError::NotPaid);
    }

    let requested_ids: Vec<Uuid> = input.lines.iter().map(|line| line.order_line_id).collect();
    let available = sqlx::query_as::<_, LineAvailability>(
        r#"
        SELECT line.id, line.quantity,
               COALESCE((
                   SELECT sum(fulfilled.quantity)::bigint
                   FROM fulfillment_lines fulfilled WHERE fulfilled.order_line_id = line.id
               ), 0) AS fulfilled_quantity
        FROM order_lines line
        WHERE line.order_id = $1 AND line.id = ANY($2)
        ORDER BY line.id
        "#,
    )
    .bind(order_id)
    .bind(&requested_ids)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    if available.len() != input.lines.len() {
        return Err(CreateError::QuantityConflict);
    }
    for requested in &input.lines {
        let Some(line) = available
            .iter()
            .find(|line| line.id == requested.order_line_id)
        else {
            return Err(CreateError::QuantityConflict);
        };
        if i64::from(requested.quantity) > i64::from(line.quantity) - line.fulfilled_quantity {
            return Err(CreateError::QuantityConflict);
        }
    }

    let fulfillment_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO fulfillments (
            id, order_id, idempotency_hash, request_hash, actor_staff_user_id,
            carrier, tracking_number, tracking_url, reason
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(fulfillment_id)
    .bind(order_id)
    .bind(idempotency_hash.as_slice())
    .bind(input.request_hash.as_slice())
    .bind(actor_id)
    .bind(&input.carrier)
    .bind(&input.tracking_number)
    .bind(&input.tracking_url)
    .bind(&input.reason)
    .execute(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    for line in &input.lines {
        sqlx::query(
            "INSERT INTO fulfillment_lines (fulfillment_id, order_line_id, quantity) VALUES ($1, $2, $3)",
        )
        .bind(fulfillment_id)
        .bind(line.order_line_id)
        .bind(line.quantity)
        .execute(&mut *transaction)
        .await
        .map_err(|_| CreateError::Database)?;
    }

    let remaining: i64 = sqlx::query_scalar(
        r#"
        SELECT sum(
            line.quantity - COALESCE((
                SELECT sum(fulfilled.quantity)::integer
                FROM fulfillment_lines fulfilled WHERE fulfilled.order_line_id = line.id
            ), 0)
        )::bigint
        FROM order_lines line
        WHERE line.order_id = $1
        "#,
    )
    .bind(order_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    let (order_status, fulfillment_status) = if remaining == 0 {
        ("completed", "fulfilled")
    } else {
        ("confirmed", "partially_fulfilled")
    };
    sqlx::query(
        "UPDATE orders SET order_status = $2, fulfillment_status = $3, updated_at = now() WHERE id = $1",
    )
    .bind(order_id)
    .bind(order_status)
    .bind(fulfillment_status)
    .execute(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    let quantity: i64 = input
        .lines
        .iter()
        .map(|line| i64::from(line.quantity))
        .sum();
    let title = if remaining == 0 {
        "Order fulfilled"
    } else {
        "Order partially fulfilled"
    };
    let detail = if input.tracking_number.is_empty() {
        format!("{quantity} item(s) marked as shipped. {}", input.reason)
    } else {
        format!(
            "{quantity} item(s) shipped with {} ({}). {}",
            input.carrier, input.tracking_number, input.reason
        )
    };
    sqlx::query(
        r#"
        INSERT INTO order_events (
            id, order_id, actor_staff_user_id, event_type, title, detail
        ) VALUES ($1, $2, $3, 'fulfillment.created', $4, $5)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(order_id)
    .bind(actor_id)
    .bind(title)
    .bind(&detail)
    .execute(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    sqlx::query(
        "INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason, metadata) VALUES ($1, 'order.fulfill', 'order', $2, $3, jsonb_build_object('fulfillment_id', $4::text, 'quantity', $5::bigint))",
    )
    .bind(actor_id)
    .bind(order_id.to_string())
    .bind(&input.reason)
    .bind(fulfillment_id.to_string())
    .bind(quantity)
    .execute(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    enqueue_fulfillment(&mut transaction, order_id, fulfillment_id)
        .await
        .map_err(|_| CreateError::Database)?;
    transaction
        .commit()
        .await
        .map_err(|_| CreateError::Database)?;
    Ok(true)
}

pub async fn load_for_order(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Vec<Fulfillment>, sqlx::Error> {
    let heads = sqlx::query_as::<_, FulfillmentHead>(
        r#"
        SELECT fulfillment.id, fulfillment.carrier, fulfillment.tracking_number,
               fulfillment.tracking_url, fulfillment.reason,
               staff.display_name AS actor_display_name,
               to_char(fulfillment.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM fulfillments fulfillment
        LEFT JOIN staff_users staff ON staff.id = fulfillment.actor_staff_user_id
        WHERE fulfillment.order_id = $1 ORDER BY fulfillment.created_at, fulfillment.id
        "#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    let mut records = Vec::with_capacity(heads.len());
    for head in heads {
        let lines = sqlx::query_as::<_, FulfillmentLine>(
            r#"
            SELECT line.id AS order_line_id, line.product_title, line.variant_title,
                   line.sku, fulfilled.quantity
            FROM fulfillment_lines fulfilled
            JOIN order_lines line ON line.id = fulfilled.order_line_id
            WHERE fulfilled.fulfillment_id = $1 ORDER BY line.position, line.id
            "#,
        )
        .bind(head.id)
        .fetch_all(pool)
        .await?;
        records.push(Fulfillment {
            id: head.id,
            carrier: head.carrier,
            tracking_number: head.tracking_number,
            tracking_url: head.tracking_url,
            reason: head.reason,
            actor_display_name: head.actor_display_name,
            lines,
            created_at: head.created_at,
        });
    }
    Ok(records)
}

fn idempotency_hash(headers: &HeaderMap) -> Option<[u8; 32]> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| {
            (16..=128).contains(&value.len())
                && value.chars().all(|character| character.is_ascii_graphic())
        })
        .map(|value| Sha256::digest(value.as_bytes()).into())
}

fn fulfillment_error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}

fn unavailable() -> Response {
    fulfillment_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "fulfillment_unavailable",
        "Fulfillment is temporarily unavailable.",
    )
}
