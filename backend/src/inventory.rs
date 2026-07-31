use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
};

const INVENTORY_ADJUST: &str = "inventory.adjust";

#[derive(Serialize, ToSchema, FromRow)]
pub struct InventoryRecord {
    pub variant_id: Uuid,
    pub variant_title: String,
    pub sku: String,
    pub product_id: Uuid,
    pub product_title: String,
    pub available_quantity: i64,
    pub reserved_quantity: i64,
    pub committed_quantity: i64,
    pub low_stock_threshold: i64,
    pub low_stock: bool,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct InventoryMovement {
    pub id: Uuid,
    pub variant_id: Uuid,
    pub actor_display_name: Option<String>,
    pub movement_type: String,
    pub quantity_delta: i64,
    pub resulting_available_quantity: i64,
    pub reason: String,
    pub created_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct AdjustInventoryRequest {
    pub quantity_delta: i64,
    pub reason: String,
    pub low_stock_threshold: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/admin/inventory",
    tag = "inventory",
    responses(
        (status = 200, body = [InventoryRecord]),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody)
    )
)]
pub async fn list(State(state): State<AppState>, actor: AuthenticatedStaff) -> Response {
    if let Err(response) = require_capability(&actor, INVENTORY_ADJUST) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match inventory_records(&pool).await {
        Ok(records) => Json(records).into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/inventory/{variant_id}/movements",
    params(("variant_id" = Uuid, Path)),
    tag = "inventory",
    responses(
        (status = 200, body = [InventoryMovement]),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody)
    )
)]
pub async fn movements(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(variant_id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_capability(&actor, INVENTORY_ADJUST) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match sqlx::query_as::<_, InventoryMovement>(
        r#"
        SELECT
            movement.id,
            movement.variant_id,
            staff.display_name AS actor_display_name,
            movement.movement_type,
            movement.quantity_delta,
            movement.resulting_available_quantity,
            movement.reason,
            to_char(movement.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM inventory_movements movement
        LEFT JOIN staff_users staff ON staff.id = movement.actor_staff_user_id
        WHERE movement.variant_id = $1
        ORDER BY movement.created_at DESC, movement.id DESC
        LIMIT 100
        "#,
    )
    .bind(variant_id)
    .fetch_all(&pool)
    .await
    {
        Ok(records) => Json(records).into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/inventory/{variant_id}/adjust",
    params(("variant_id" = Uuid, Path)),
    tag = "inventory",
    request_body = AdjustInventoryRequest,
    responses(
        (status = 200, body = InventoryRecord),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 422, body = ErrorBody)
    )
)]
pub async fn adjust(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(variant_id): Path<Uuid>,
    Json(input): Json<AdjustInventoryRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, INVENTORY_ADJUST) {
        return response.into_response();
    }
    let reason = input.reason.trim();
    if input.quantity_delta == 0
        || reason.len() < 3
        || reason.len() > 500
        || input.low_stock_threshold.is_some_and(|value| value < 0)
    {
        return invalid_input();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let current: Option<i64> = match sqlx::query_scalar(
        "SELECT available_quantity FROM inventory_items WHERE variant_id = $1 FOR UPDATE",
    )
    .bind(variant_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let Some(current) = current else {
        return not_found();
    };
    let Some(resulting) = current.checked_add(input.quantity_delta) else {
        return invalid_input();
    };
    if resulting < 0 {
        return insufficient_stock();
    }
    if sqlx::query(
        r#"
        UPDATE inventory_items
        SET available_quantity = $2,
            low_stock_threshold = COALESCE($3, low_stock_threshold),
            updated_at = now()
        WHERE variant_id = $1
        "#,
    )
    .bind(variant_id)
    .bind(resulting)
    .bind(input.low_stock_threshold)
    .execute(&mut *transaction)
    .await
    .is_err()
    {
        return unavailable();
    }
    let movement_id = Uuid::now_v7();
    if sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, variant_id, actor_staff_user_id, movement_type,
            quantity_delta, resulting_available_quantity, reason
        )
        VALUES ($1, $2, $3, 'adjustment', $4, $5, $6)
        "#,
    )
    .bind(movement_id)
    .bind(variant_id)
    .bind(actor.id)
    .bind(input.quantity_delta)
    .bind(resulting)
    .bind(reason)
    .execute(&mut *transaction)
    .await
    .is_err()
        || sqlx::query(
            r#"
            INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason)
            VALUES ($1, 'inventory.adjust', 'inventory_item', $2, $3)
            "#,
        )
        .bind(actor.id)
        .bind(variant_id.to_string())
        .bind(reason)
        .execute(&mut *transaction)
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    match inventory_record(&pool, variant_id).await {
        Ok(Some(record)) => Json(record).into_response(),
        Ok(None) => not_found(),
        Err(_) => unavailable(),
    }
}

async fn inventory_records(pool: &PgPool) -> Result<Vec<InventoryRecord>, sqlx::Error> {
    let query = format!(
        "{} ORDER BY product.title, variant.position, variant.id",
        inventory_query()
    );
    sqlx::query_as(&query).fetch_all(pool).await
}

async fn inventory_record(
    pool: &PgPool,
    variant_id: Uuid,
) -> Result<Option<InventoryRecord>, sqlx::Error> {
    let query = format!("{} AND variant.id = $1", inventory_query());
    sqlx::query_as(&query)
        .bind(variant_id)
        .fetch_optional(pool)
        .await
}

fn inventory_query() -> &'static str {
    r#"
    SELECT
        variant.id AS variant_id,
        variant.title AS variant_title,
        variant.sku,
        product.id AS product_id,
        product.title AS product_title,
        inventory.available_quantity,
        inventory.reserved_quantity,
        inventory.committed_quantity,
        inventory.low_stock_threshold,
        inventory.available_quantity <= inventory.low_stock_threshold AS low_stock
    FROM inventory_items inventory
    JOIN product_variants variant ON variant.id = inventory.variant_id
    JOIN products product ON product.id = variant.product_id
    WHERE true
    "#
}

fn invalid_input() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorBody::new(
            "invalid_inventory_adjustment",
            "Use a non-zero quantity, a reason of 3–500 characters, and a non-negative threshold.",
        )),
    )
        .into_response()
}

fn insufficient_stock() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorBody::new(
            "insufficient_stock",
            "The adjustment would make available stock negative.",
        )),
    )
        .into_response()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody::new(
            "inventory_not_found",
            "Inventory for the variant was not found.",
        )),
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody::new(
            "inventory_unavailable",
            "Inventory is temporarily unavailable.",
        )),
    )
        .into_response()
}
