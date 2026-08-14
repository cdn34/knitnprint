use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
    inventory::{InventoryOperationError, release_in_transaction, restock_in_transaction},
    orders::{Order, load_order},
    payments::ProviderRefundRequest,
};

const ORDERS_REFUND: &str = "orders.refund";

#[derive(Deserialize, Serialize, ToSchema)]
pub struct CancelOrderRequest {
    pub reason: String,
    #[serde(default)]
    pub internal_note: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct CreateRefundRequest {
    pub mode: String,
    #[serde(default)]
    pub lines: Vec<CreateRefundLineRequest>,
    pub restock: bool,
    pub reason: String,
    #[serde(default)]
    pub internal_note: String,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateRefundLineRequest {
    pub order_line_id: Uuid,
    pub quantity: i32,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct Refund {
    pub id: Uuid,
    pub provider: String,
    pub provider_refund_id: Option<String>,
    pub status: String,
    pub mode: String,
    pub amount_minor: i64,
    pub currency: String,
    pub restock: bool,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_note: Option<String>,
    pub actor_display_name: Option<String>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    #[sqlx(skip)]
    pub lines: Vec<RefundLine>,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct RefundLine {
    pub order_line_id: Uuid,
    pub product_title: String,
    pub variant_title: String,
    pub sku: String,
    pub quantity: i32,
    pub amount_minor: i64,
}

#[derive(Serialize, ToSchema)]
pub struct OrderOperations {
    pub can_cancel: bool,
    pub cancel_unavailable_reason: Option<String>,
    pub can_refund: bool,
    pub refund_unavailable_reason: Option<String>,
    pub refundable_minor: i64,
}

#[derive(FromRow)]
struct LockedOrder {
    order_status: String,
    payment_status: String,
    fulfillment_status: String,
    order_number: String,
    payment_id: Uuid,
    provider: String,
    provider_payment_id: Option<String>,
    provider_charge_id: Option<String>,
    amount_minor: i64,
    currency: String,
}

#[derive(FromRow)]
struct RefundableLine {
    id: Uuid,
    quantity: i32,
    unit_price_minor: i64,
    already_refunded: i64,
}

struct PreparedRefund {
    refund_id: Uuid,
    order_id: Uuid,
    provider: String,
    provider_charge_id: Option<String>,
    amount_minor: i64,
    reason: String,
    replay: bool,
}

#[utoipa::path(
    post,
    path = "/api/admin/orders/{order_id}/cancel",
    tag = "admin orders",
    params(("order_id" = Uuid, Path), ("Idempotency-Key" = String, Header)),
    request_body = CancelOrderRequest,
    responses(
        (status = 200, body = Order),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn cancel(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CancelOrderRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, ORDERS_REFUND) {
        return response.into_response();
    }
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_idempotency_key",
            "Provide a 16–128 character Idempotency-Key.",
        );
    };
    let Ok(input) = ValidatedCancellation::new(input) else {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_cancellation",
            "Provide a reason of 3–500 characters and an internal note no longer than 2,000 characters.",
        );
    };
    let Some(pool) = state.database else {
        return unavailable();
    };

    let existing = cancellation_request_hash(&pool, order_id, idempotency_hash).await;
    match existing {
        Ok(Some(hash)) if hash.as_slice() == input.request_hash => {
            return order_response(&pool, order_id).await;
        }
        Ok(Some(_)) => {
            return error(
                StatusCode::CONFLICT,
                "cancellation_idempotency_conflict",
                "That cancellation key was already used for a different request.",
            );
        }
        Err(_) => return unavailable(),
        Ok(None) => {}
    }

    let candidate = match load_locked_order(&pool, order_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return error(
                StatusCode::NOT_FOUND,
                "order_not_found",
                "The order was not found.",
            );
        }
        Err(_) => return unavailable(),
    };
    if !cancellable(&candidate) {
        return error(
            StatusCode::CONFLICT,
            "order_not_cancellable",
            "Only unpaid orders with no fulfillment can be cancelled.",
        );
    }
    if candidate.provider == "stripe"
        && let Some(checkout_id) = candidate.provider_payment_id.as_ref()
    {
        let Some(provider) = state.payments.provider() else {
            return unavailable();
        };
        if provider.cancel_checkout(checkout_id.clone()).await.is_err() {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "payment_provider_unavailable",
                "Stripe checkout could not be cancelled. The order was left unchanged.",
            );
        }
    }
    match cancel_in_transaction(&pool, order_id, actor.id, idempotency_hash, &input).await {
        Ok(()) => order_response(&pool, order_id).await,
        Err(OperationError::NotFound) => error(
            StatusCode::NOT_FOUND,
            "order_not_found",
            "The order was not found.",
        ),
        Err(OperationError::Conflict) => error(
            StatusCode::CONFLICT,
            "order_not_cancellable",
            "Only unpaid orders with no fulfillment can be cancelled.",
        ),
        Err(OperationError::Idempotency) => error(
            StatusCode::CONFLICT,
            "cancellation_idempotency_conflict",
            "That cancellation key was already used for a different request.",
        ),
        Err(OperationError::Database) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/orders/{order_id}/refunds",
    tag = "admin orders",
    params(("order_id" = Uuid, Path), ("Idempotency-Key" = String, Header)),
    request_body = CreateRefundRequest,
    responses(
        (status = 200, body = Order),
        (status = 202, body = Order),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn refund(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(order_id): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CreateRefundRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, ORDERS_REFUND) {
        return response.into_response();
    }
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_idempotency_key",
            "Provide a 16–128 character Idempotency-Key.",
        );
    };
    let Ok(input) = ValidatedRefund::new(input) else {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_refund",
            "Choose full or partial, provide valid line quantities, a restocking decision, and a reason of 3–500 characters.",
        );
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let prepared = match prepare_refund(&pool, order_id, actor.id, idempotency_hash, &input).await {
        Ok(value) => value,
        Err(OperationError::NotFound) => {
            return error(
                StatusCode::NOT_FOUND,
                "order_not_found",
                "The order was not found.",
            );
        }
        Err(OperationError::Conflict) => {
            return error(
                StatusCode::CONFLICT,
                "order_not_refundable",
                "The payment or requested quantities are not refundable.",
            );
        }
        Err(OperationError::Idempotency) => {
            return error(
                StatusCode::CONFLICT,
                "refund_idempotency_conflict",
                "That refund key was already used for a different request.",
            );
        }
        Err(OperationError::Database) => return unavailable(),
    };
    if prepared.replay {
        return order_response(&pool, order_id).await;
    }
    if prepared.provider == "manual" {
        let manual_reference = format!("manual-{}", prepared.refund_id);
        if finalize_refund(
            &pool,
            prepared.refund_id,
            &manual_reference,
            "succeeded",
            None,
        )
        .await
        .is_err()
        {
            return unavailable();
        }
        return order_response(&pool, order_id).await;
    }
    let Some(charge_id) = prepared.provider_charge_id.clone() else {
        fail_refund(
            &pool,
            prepared.refund_id,
            "missing_payment_reference",
            "Stripe payment reference is unavailable.",
        )
        .await
        .ok();
        return error(
            StatusCode::CONFLICT,
            "refund_reference_unavailable",
            "This Stripe payment does not have a refundable payment reference.",
        );
    };
    let Some(provider) = state.payments.provider() else {
        return unavailable();
    };
    let provider_result = provider
        .refund(ProviderRefundRequest {
            refund_id: prepared.refund_id,
            order_id: prepared.order_id,
            provider_charge_id: charge_id,
            amount_minor: prepared.amount_minor,
            reason: prepared.reason,
        })
        .await;
    let result = match provider_result {
        Ok(result) => result,
        Err(_) => {
            fail_refund(
                &pool,
                prepared.refund_id,
                "provider_error",
                "Stripe rejected or could not process the refund request.",
            )
            .await
            .ok();
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "payment_provider_unavailable",
                "Stripe could not process the refund. No inventory was restocked.",
            );
        }
    };
    if finalize_refund(
        &pool,
        prepared.refund_id,
        &result.provider_refund_id,
        &result.status,
        None,
    )
    .await
    .is_err()
    {
        return unavailable();
    }
    let status = if result.status == "succeeded" {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    match load_order(&pool, order_id).await {
        Ok(Some(order)) => (status, Json(order)).into_response(),
        _ => unavailable(),
    }
}

struct ValidatedCancellation {
    reason: String,
    internal_note: String,
    request_hash: [u8; 32],
}

impl ValidatedCancellation {
    fn new(input: CancelOrderRequest) -> Result<Self, ()> {
        let reason = input.reason.trim().to_owned();
        let internal_note = input.internal_note.trim().to_owned();
        if !(3..=500).contains(&reason.len()) || internal_note.len() > 2000 {
            return Err(());
        }
        let request_hash =
            Sha256::digest(serde_json::to_vec(&(&reason, &internal_note)).map_err(|_| ())?).into();
        Ok(Self {
            reason,
            internal_note,
            request_hash,
        })
    }
}

struct ValidatedRefund {
    mode: String,
    lines: Vec<CreateRefundLineRequest>,
    restock: bool,
    reason: String,
    internal_note: String,
    request_hash: [u8; 32],
}

impl ValidatedRefund {
    fn new(input: CreateRefundRequest) -> Result<Self, ()> {
        let mode = input.mode.trim().to_ascii_lowercase();
        let reason = input.reason.trim().to_owned();
        let internal_note = input.internal_note.trim().to_owned();
        let unique: HashSet<_> = input.lines.iter().map(|line| line.order_line_id).collect();
        if !matches!(mode.as_str(), "full" | "partial")
            || (mode == "full" && !input.lines.is_empty())
            || (mode == "partial" && input.lines.is_empty())
            || unique.len() != input.lines.len()
            || input
                .lines
                .iter()
                .any(|line| !(1..=99).contains(&line.quantity))
            || !(3..=500).contains(&reason.len())
            || internal_note.len() > 2000
        {
            return Err(());
        }
        let request_hash = Sha256::digest(
            serde_json::to_vec(&(&mode, &input.lines, input.restock, &reason, &internal_note))
                .map_err(|_| ())?,
        )
        .into();
        Ok(Self {
            mode,
            lines: input.lines,
            restock: input.restock,
            reason,
            internal_note,
            request_hash,
        })
    }
}

enum OperationError {
    NotFound,
    Conflict,
    Idempotency,
    Database,
}

async fn load_locked_order(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Option<LockedOrder>, sqlx::Error> {
    sqlx::query_as(
        r#"SELECT orders.order_status, orders.payment_status, orders.fulfillment_status,
                  orders.order_number, payment.id AS payment_id, payment.provider,
                  payment.provider_payment_id, payment.provider_charge_id,
                  payment.amount_minor, payment.currency::text AS currency
           FROM orders JOIN order_payments payment ON payment.order_id = orders.id
           WHERE orders.id = $1"#,
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await
}

fn cancellable(order: &LockedOrder) -> bool {
    order.order_status == "pending"
        && order.payment_status == "pending"
        && order.fulfillment_status == "unfulfilled"
}

async fn cancellation_request_hash(
    pool: &PgPool,
    order_id: Uuid,
    hash: [u8; 32],
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    sqlx::query_scalar("SELECT request_hash FROM order_cancellations WHERE order_id = $1 AND idempotency_hash = $2")
        .bind(order_id).bind(hash.as_slice()).fetch_optional(pool).await
}

async fn cancel_in_transaction(
    pool: &PgPool,
    order_id: Uuid,
    actor_id: Uuid,
    idempotency_hash: [u8; 32],
    input: &ValidatedCancellation,
) -> Result<(), OperationError> {
    let mut tx = pool.begin().await.map_err(|_| OperationError::Database)?;
    let order = sqlx::query_as::<_, LockedOrder>(
        r#"SELECT orders.order_status, orders.payment_status, orders.fulfillment_status,
                  orders.order_number, payment.id AS payment_id, payment.provider,
                  payment.provider_payment_id, payment.provider_charge_id,
                  payment.amount_minor, payment.currency::text AS currency
           FROM orders JOIN order_payments payment ON payment.order_id = orders.id
           WHERE orders.id = $1 FOR UPDATE OF orders, payment"#,
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| OperationError::Database)?
    .ok_or(OperationError::NotFound)?;
    let existing: Option<Vec<u8>> = sqlx::query_scalar("SELECT request_hash FROM order_cancellations WHERE order_id = $1 AND idempotency_hash = $2")
        .bind(order_id).bind(idempotency_hash.as_slice()).fetch_optional(&mut *tx).await.map_err(|_| OperationError::Database)?;
    if let Some(existing) = existing {
        if existing.as_slice() != input.request_hash {
            return Err(OperationError::Idempotency);
        }
        tx.commit().await.map_err(|_| OperationError::Database)?;
        return Ok(());
    }
    if !cancellable(&order) {
        return Err(OperationError::Conflict);
    }
    let lines: Vec<(Uuid, i32)> = sqlx::query_as(
        "SELECT variant_id, quantity FROM order_lines WHERE order_id = $1 ORDER BY variant_id",
    )
    .bind(order_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| OperationError::Database)?;
    for (variant_id, quantity) in lines {
        release_in_transaction(
            &mut tx,
            variant_id,
            i64::from(quantity),
            &format!(
                "Released after staff cancelled order {}",
                order.order_number
            ),
        )
        .await
        .map_err(|_| OperationError::Database)?;
    }
    sqlx::query("UPDATE orders SET order_status = 'cancelled', payment_status = 'cancelled', updated_at = now() WHERE id = $1")
        .bind(order_id).execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    sqlx::query("UPDATE order_payments SET status = 'cancelled', failure_code = NULL, failure_message = NULL, updated_at = now() WHERE id = $1")
        .bind(order.payment_id).execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    sqlx::query("UPDATE payment_attempts SET status = 'cancelled', updated_at = now() WHERE order_payment_id = $1 AND status IN ('creating', 'pending', 'processing')")
        .bind(order.payment_id).execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    sqlx::query("INSERT INTO order_cancellations (id, order_id, actor_staff_user_id, idempotency_hash, request_hash, reason, internal_note) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(Uuid::now_v7()).bind(order_id).bind(actor_id).bind(idempotency_hash.as_slice()).bind(input.request_hash.as_slice()).bind(&input.reason).bind(&input.internal_note)
        .execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    sqlx::query("INSERT INTO payment_status_events (id, order_payment_id, provider, event_type, provider_status, detail) VALUES ($1,$2,$3,'payment.cancelled','cancelled',$4)")
        .bind(Uuid::now_v7()).bind(order.payment_id).bind(&order.provider).bind(&input.reason).execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    insert_event(
        &mut tx,
        order_id,
        actor_id,
        "order.cancelled",
        "Order cancelled",
        &input.reason,
    )
    .await
    .map_err(|_| OperationError::Database)?;
    sqlx::query("INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason, metadata) VALUES ($1,'order.cancel','order',$2,$3,jsonb_build_object('internal_note',$4::text))")
        .bind(actor_id).bind(order_id.to_string()).bind(&input.reason).bind(&input.internal_note).execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    tx.commit().await.map_err(|_| OperationError::Database)
}

async fn prepare_refund(
    pool: &PgPool,
    order_id: Uuid,
    actor_id: Uuid,
    idempotency_hash: [u8; 32],
    input: &ValidatedRefund,
) -> Result<PreparedRefund, OperationError> {
    let mut tx = pool.begin().await.map_err(|_| OperationError::Database)?;
    let order = sqlx::query_as::<_, LockedOrder>(
        r#"SELECT orders.order_status, orders.payment_status, orders.fulfillment_status,
                  orders.order_number, payment.id AS payment_id, payment.provider,
                  payment.provider_payment_id, payment.provider_charge_id,
                  payment.amount_minor, payment.currency::text AS currency
           FROM orders JOIN order_payments payment ON payment.order_id = orders.id
           WHERE orders.id = $1 FOR UPDATE OF orders, payment"#,
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| OperationError::Database)?
    .ok_or(OperationError::NotFound)?;
    let existing: Option<(Uuid, Vec<u8>, String, i64, String)> = sqlx::query_as("SELECT id, request_hash, status, amount_minor, reason FROM order_refunds WHERE order_id = $1 AND idempotency_hash = $2")
        .bind(order_id).bind(idempotency_hash.as_slice()).fetch_optional(&mut *tx).await.map_err(|_| OperationError::Database)?;
    if let Some((id, request_hash, _status, amount_minor, reason)) = existing {
        if request_hash.as_slice() != input.request_hash {
            return Err(OperationError::Idempotency);
        }
        tx.commit().await.map_err(|_| OperationError::Database)?;
        return Ok(PreparedRefund {
            refund_id: id,
            order_id,
            provider: order.provider,
            provider_charge_id: order.provider_charge_id,
            amount_minor,
            reason,
            replay: true,
        });
    }
    if !matches!(order.payment_status.as_str(), "paid" | "partially_refunded")
        || sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM order_refunds WHERE order_id = $1 AND status = 'pending')")
            .bind(order_id).fetch_one(&mut *tx).await.map_err(|_| OperationError::Database)?
    { return Err(OperationError::Conflict) }
    let lines = sqlx::query_as::<_, RefundableLine>(
        r#"SELECT line.id, line.quantity, line.unit_price_minor,
                  COALESCE((SELECT sum(refund_line.quantity)::bigint FROM order_refund_lines refund_line
                    JOIN order_refunds refund ON refund.id = refund_line.refund_id
                    WHERE refund_line.order_line_id = line.id AND refund.status = 'succeeded'), 0) AS already_refunded
           FROM order_lines line WHERE line.order_id = $1 ORDER BY line.position, line.id"#,
    ).bind(order_id).fetch_all(&mut *tx).await.map_err(|_| OperationError::Database)?;
    let succeeded_minor: i64 = sqlx::query_scalar("SELECT COALESCE(sum(amount_minor),0)::bigint FROM order_refunds WHERE order_id = $1 AND status = 'succeeded'")
        .bind(order_id).fetch_one(&mut *tx).await.map_err(|_| OperationError::Database)?;
    let remaining_minor = order
        .amount_minor
        .checked_sub(succeeded_minor)
        .filter(|v| *v > 0)
        .ok_or(OperationError::Conflict)?;
    let selected: Vec<(&RefundableLine, i32)> = if input.mode == "full" {
        lines
            .iter()
            .filter_map(|line| {
                let quantity = i64::from(line.quantity) - line.already_refunded;
                (quantity > 0).then_some((line, i32::try_from(quantity).ok()?))
            })
            .collect()
    } else {
        let mut result = Vec::with_capacity(input.lines.len());
        for requested in &input.lines {
            let line = lines
                .iter()
                .find(|line| line.id == requested.order_line_id)
                .ok_or(OperationError::Conflict)?;
            if i64::from(requested.quantity) > i64::from(line.quantity) - line.already_refunded {
                return Err(OperationError::Conflict);
            }
            result.push((line, requested.quantity));
        }
        result
    };
    if selected.is_empty() {
        return Err(OperationError::Conflict);
    }
    let amount_minor = if input.mode == "full" {
        remaining_minor
    } else {
        selected
            .iter()
            .try_fold(0_i64, |sum, (line, quantity)| {
                line.unit_price_minor
                    .checked_mul(i64::from(*quantity))
                    .and_then(|value| sum.checked_add(value))
            })
            .ok_or(OperationError::Conflict)?
    };
    if amount_minor <= 0 || amount_minor > remaining_minor {
        return Err(OperationError::Conflict);
    }
    let refund_id = Uuid::now_v7();
    sqlx::query("INSERT INTO order_refunds (id,order_id,order_payment_id,actor_staff_user_id,idempotency_hash,request_hash,provider,mode,amount_minor,currency,restock,reason,internal_note) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
        .bind(refund_id).bind(order_id).bind(order.payment_id).bind(actor_id).bind(idempotency_hash.as_slice()).bind(input.request_hash.as_slice()).bind(&order.provider).bind(&input.mode).bind(amount_minor).bind(&order.currency).bind(input.restock).bind(&input.reason).bind(&input.internal_note)
        .execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    for (line, quantity) in selected {
        sqlx::query("INSERT INTO order_refund_lines (refund_id,order_line_id,quantity,amount_minor) VALUES ($1,$2,$3,$4)")
            .bind(refund_id).bind(line.id).bind(quantity).bind(line.unit_price_minor * i64::from(quantity)).execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    }
    insert_event(
        &mut tx,
        order_id,
        actor_id,
        "payment.refund_requested",
        "Refund requested",
        &input.reason,
    )
    .await
    .map_err(|_| OperationError::Database)?;
    sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,reason,metadata) VALUES ($1,'order.refund_requested','order',$2,$3,jsonb_build_object('refund_id',$4::text,'amount_minor',$5::bigint,'restock',$6::boolean,'internal_note',$7::text))")
        .bind(actor_id).bind(order_id.to_string()).bind(&input.reason).bind(refund_id.to_string()).bind(amount_minor).bind(input.restock).bind(&input.internal_note)
        .execute(&mut *tx).await.map_err(|_| OperationError::Database)?;
    tx.commit().await.map_err(|_| OperationError::Database)?;
    Ok(PreparedRefund {
        refund_id,
        order_id,
        provider: order.provider,
        provider_charge_id: order.provider_charge_id,
        amount_minor,
        reason: input.reason.clone(),
        replay: false,
    })
}

#[allow(clippy::type_complexity)]
pub async fn finalize_refund(
    pool: &PgPool,
    refund_id: Uuid,
    provider_refund_id: &str,
    provider_status: &str,
    provider_event_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    if !matches!(provider_status, "succeeded" | "failed" | "pending") {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    let record: Option<(Uuid, Uuid, Uuid, String, String, i64, bool, String, Option<String>, String)> = sqlx::query_as(
        "SELECT refund.order_id, refund.order_payment_id, refund.actor_staff_user_id, refund.status, refund.mode, refund.amount_minor, refund.restock, refund.reason, refund.provider_refund_id, orders.fulfillment_status FROM order_refunds refund JOIN orders ON orders.id = refund.order_id WHERE refund.id = $1 FOR UPDATE OF refund, orders"
    ).bind(refund_id).fetch_optional(&mut *tx).await?;
    let Some((
        order_id,
        payment_id,
        actor_id,
        current_status,
        mode,
        amount_minor,
        restock,
        reason,
        existing_reference,
        fulfillment_status,
    )) = record
    else {
        tx.commit().await?;
        return Ok(());
    };
    if current_status == "succeeded" || current_status == "failed" {
        tx.commit().await?;
        return Ok(());
    }
    if existing_reference
        .as_deref()
        .is_some_and(|value| value != provider_refund_id)
    {
        tx.commit().await?;
        return Ok(());
    }
    if provider_status == "pending" {
        sqlx::query(
            "UPDATE order_refunds SET provider_refund_id = $2, updated_at = now() WHERE id = $1",
        )
        .bind(refund_id)
        .bind(provider_refund_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Ok(());
    }
    if provider_status == "failed" {
        sqlx::query("UPDATE order_refunds SET provider_refund_id = $2, status = 'failed', failure_code = 'provider_failed', failure_message = 'The payment provider reported a failed refund.', updated_at = now() WHERE id = $1")
            .bind(refund_id).bind(provider_refund_id).execute(&mut *tx).await?;
        tx.commit().await?;
        return Ok(());
    }
    if restock {
        let lines: Vec<(Uuid, i32)> = sqlx::query_as("SELECT line.variant_id, refunded.quantity FROM order_refund_lines refunded JOIN order_lines line ON line.id = refunded.order_line_id WHERE refunded.refund_id = $1 ORDER BY line.variant_id")
            .bind(refund_id).fetch_all(&mut *tx).await?;
        for (variant_id, quantity) in lines {
            restock_in_transaction(
                &mut tx,
                variant_id,
                i64::from(quantity),
                &format!("Restocked after refund for order {order_id}"),
            )
            .await
            .map_err(inventory_error)?;
        }
    }
    sqlx::query("UPDATE order_refunds SET provider_refund_id = $2, status = 'succeeded', completed_at = now(), updated_at = now() WHERE id = $1")
        .bind(refund_id).bind(provider_refund_id).execute(&mut *tx).await?;
    let payment_total: i64 =
        sqlx::query_scalar("SELECT amount_minor FROM order_payments WHERE id = $1")
            .bind(payment_id)
            .fetch_one(&mut *tx)
            .await?;
    let refunded_total: i64 = sqlx::query_scalar("SELECT COALESCE(sum(amount_minor),0)::bigint FROM order_refunds WHERE order_payment_id = $1 AND status = 'succeeded'")
        .bind(payment_id).fetch_one(&mut *tx).await?;
    let payment_status = if refunded_total == payment_total {
        "refunded"
    } else {
        "partially_refunded"
    };
    sqlx::query("UPDATE order_payments SET status = $2, updated_at = now() WHERE id = $1")
        .bind(payment_id)
        .bind(payment_status)
        .execute(&mut *tx)
        .await?;
    let order_status = if payment_status == "refunded" && fulfillment_status == "unfulfilled" {
        Some("cancelled")
    } else {
        None
    };
    sqlx::query("UPDATE orders SET payment_status = $2, order_status = COALESCE($3, order_status), updated_at = now() WHERE id = $1")
        .bind(order_id).bind(payment_status).bind(order_status).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO payment_status_events (id,order_payment_id,provider,provider_event_id,event_type,provider_status,detail) SELECT $1,$2,provider,$3,'payment.refund_succeeded','succeeded',$4 FROM order_payments WHERE id = $2")
        .bind(Uuid::now_v7()).bind(payment_id).bind(provider_event_id).bind(format!("Refunded {amount_minor} minor units.")).execute(&mut *tx).await?;
    let title = if mode == "full" {
        "Payment refunded"
    } else {
        "Payment partially refunded"
    };
    insert_event(
        &mut tx,
        order_id,
        actor_id,
        "payment.refunded",
        title,
        &reason,
    )
    .await?;
    sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,reason,metadata) VALUES ($1,'order.refund','order',$2,$3,jsonb_build_object('refund_id',$4::text,'amount_minor',$5::bigint,'restock',$6::boolean))")
        .bind(actor_id).bind(order_id.to_string()).bind(&reason).bind(refund_id.to_string()).bind(amount_minor).bind(restock).execute(&mut *tx).await?;
    tx.commit().await
}

async fn fail_refund(
    pool: &PgPool,
    refund_id: Uuid,
    code: &str,
    message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE order_refunds SET status = 'failed', failure_code = $2, failure_message = $3, updated_at = now() WHERE id = $1 AND status = 'pending'")
        .bind(refund_id).bind(code).bind(message).execute(pool).await?;
    Ok(())
}

fn inventory_error(error: InventoryOperationError) -> sqlx::Error {
    match error {
        InventoryOperationError::Database(error) => error,
        other => sqlx::Error::Protocol(other.to_string()),
    }
}

async fn insert_event(
    tx: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    actor_id: Uuid,
    event_type: &str,
    title: &str,
    detail: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO order_events (id,order_id,actor_staff_user_id,event_type,title,detail) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(Uuid::now_v7()).bind(order_id).bind(actor_id).bind(event_type).bind(title).bind(detail).execute(&mut **tx).await?;
    Ok(())
}

pub async fn load_refunds(pool: &PgPool, order_id: Uuid) -> Result<Vec<Refund>, sqlx::Error> {
    let mut refunds = sqlx::query_as::<_, Refund>(
        r#"SELECT refund.id, refund.provider, refund.provider_refund_id, refund.status, refund.mode,
                  refund.amount_minor, refund.currency::text AS currency, refund.restock, refund.reason,
                  refund.internal_note, staff.display_name AS actor_display_name,
                  refund.failure_code, refund.failure_message,
                  CASE WHEN refund.completed_at IS NULL THEN NULL ELSE to_char(refund.completed_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') END AS completed_at,
                  to_char(refund.created_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
           FROM order_refunds refund LEFT JOIN staff_users staff ON staff.id = refund.actor_staff_user_id
           WHERE refund.order_id = $1 ORDER BY refund.created_at, refund.id"#,
    ).bind(order_id).fetch_all(pool).await?;
    for refund in &mut refunds {
        refund.lines = sqlx::query_as::<_, RefundLine>(
            "SELECT line.id AS order_line_id,line.product_title,line.variant_title,line.sku,refunded.quantity,refunded.amount_minor FROM order_refund_lines refunded JOIN order_lines line ON line.id = refunded.order_line_id WHERE refunded.refund_id = $1 ORDER BY line.position,line.id"
        ).bind(refund.id).fetch_all(pool).await?;
    }
    Ok(refunds)
}

pub async fn load_operations(
    pool: &PgPool,
    order_id: Uuid,
    order_status: &str,
    payment_status: &str,
    fulfillment_status: &str,
    payment_total: i64,
) -> Result<OrderOperations, sqlx::Error> {
    let (refunded, pending): (i64, bool) = sqlx::query_as("SELECT COALESCE(sum(amount_minor) FILTER (WHERE status IN ('pending','succeeded')),0)::bigint, COALESCE(bool_or(status = 'pending'), false) FROM order_refunds WHERE order_id = $1")
        .bind(order_id).fetch_one(pool).await?;
    let refundable_minor = payment_total.saturating_sub(refunded);
    let can_cancel = order_status == "pending"
        && payment_status == "pending"
        && fulfillment_status == "unfulfilled";
    let can_refund =
        matches!(payment_status, "paid" | "partially_refunded") && refundable_minor > 0 && !pending;
    Ok(OrderOperations {
        can_cancel,
        cancel_unavailable_reason: (!can_cancel)
            .then(|| "Only unpaid orders with no fulfillment can be cancelled.".to_owned()),
        can_refund,
        refund_unavailable_reason: (!can_refund)
            .then(|| "A paid balance is required before a refund can be created.".to_owned()),
        refundable_minor,
    })
}

fn idempotency_hash(headers: &HeaderMap) -> Option<[u8; 32]> {
    headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| (16..=128).contains(&value.len()))
        .map(|value| Sha256::digest(value.as_bytes()).into())
}

async fn order_response(pool: &PgPool, order_id: Uuid) -> Response {
    match load_order(pool, order_id).await {
        Ok(Some(order)) => Json(order).into_response(),
        Ok(None) => error(
            StatusCode::NOT_FOUND,
            "order_not_found",
            "The order was not found.",
        ),
        Err(_) => unavailable(),
    }
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "order_operation_unavailable",
        "The order operation is temporarily unavailable.",
    )
}
