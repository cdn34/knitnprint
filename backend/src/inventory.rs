use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use thiserror::Error;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRow)]
pub struct Availability {
    pub variant_id: Uuid,
    pub available_quantity: i64,
    pub reserved_quantity: i64,
    pub committed_quantity: i64,
    pub low_stock_threshold: i64,
    pub low_stock: bool,
}

#[derive(Debug, Error)]
pub enum InventoryOperationError {
    #[error("inventory quantity must be positive")]
    InvalidQuantity,
    #[error("inventory reason must contain 3–500 characters")]
    InvalidReason,
    #[error("low-stock threshold must be non-negative")]
    InvalidThreshold,
    #[error("inventory for the variant was not found")]
    NotFound,
    #[error("available stock is insufficient")]
    InsufficientAvailable,
    #[error("reserved stock is insufficient")]
    InsufficientReserved,
    #[error("inventory quantity is outside the supported range")]
    QuantityOverflow,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(FromRow)]
struct InventoryQuantities {
    available_quantity: i64,
    reserved_quantity: i64,
    committed_quantity: i64,
    low_stock_threshold: i64,
}

#[derive(Clone, Copy)]
enum StockTransition {
    Reserve,
    Release,
    Commit,
    Restock,
}

impl StockTransition {
    const fn movement_type(self) -> &'static str {
        match self {
            Self::Reserve => "reservation",
            Self::Release => "release",
            Self::Commit => "commitment",
            Self::Restock => "restock",
        }
    }
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
    if validate_adjustment(
        input.quantity_delta,
        input.low_stock_threshold,
        &input.reason,
    )
    .is_err()
    {
        return invalid_input();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match adjust_stock(
        &pool,
        variant_id,
        input.quantity_delta,
        input.low_stock_threshold,
        &input.reason,
        actor.id,
    )
    .await
    {
        Ok(_) => {}
        Err(
            InventoryOperationError::InvalidQuantity
            | InventoryOperationError::InvalidReason
            | InventoryOperationError::InvalidThreshold
            | InventoryOperationError::QuantityOverflow,
        ) => return invalid_input(),
        Err(InventoryOperationError::InsufficientAvailable) => return insufficient_stock(),
        Err(InventoryOperationError::NotFound) => return not_found(),
        Err(
            InventoryOperationError::InsufficientReserved | InventoryOperationError::Database(_),
        ) => return unavailable(),
    }
    match inventory_record(&pool, variant_id).await {
        Ok(Some(record)) => Json(record).into_response(),
        Ok(None) => not_found(),
        Err(_) => unavailable(),
    }
}

pub async fn get_availability(
    pool: &PgPool,
    variant_id: Uuid,
) -> Result<Option<Availability>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT
            variant_id,
            available_quantity,
            reserved_quantity,
            committed_quantity,
            low_stock_threshold,
            available_quantity <= low_stock_threshold AS low_stock
        FROM inventory_items
        WHERE variant_id = $1
        "#,
    )
    .bind(variant_id)
    .fetch_optional(pool)
    .await
}

pub async fn adjust_stock(
    pool: &PgPool,
    variant_id: Uuid,
    quantity_delta: i64,
    low_stock_threshold: Option<i64>,
    reason: &str,
    actor_staff_user_id: Uuid,
) -> Result<Availability, InventoryOperationError> {
    let reason = validate_adjustment(quantity_delta, low_stock_threshold, reason)?;

    let mut transaction = pool.begin().await?;
    let current = locked_quantities(&mut transaction, variant_id).await?;
    let resulting_available = current
        .available_quantity
        .checked_add(quantity_delta)
        .ok_or(InventoryOperationError::QuantityOverflow)?;
    if resulting_available < 0 {
        return Err(InventoryOperationError::InsufficientAvailable);
    }
    let resulting_threshold = low_stock_threshold.unwrap_or(current.low_stock_threshold);

    sqlx::query(
        r#"
        UPDATE inventory_items
        SET available_quantity = $2,
            low_stock_threshold = $3,
            updated_at = now()
        WHERE variant_id = $1
        "#,
    )
    .bind(variant_id)
    .bind(resulting_available)
    .bind(resulting_threshold)
    .execute(&mut *transaction)
    .await?;
    insert_movement(
        &mut transaction,
        variant_id,
        Some(actor_staff_user_id),
        "adjustment",
        quantity_delta,
        resulting_available,
        reason,
    )
    .await?;
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason)
        VALUES ($1, 'inventory.adjust', 'inventory_item', $2, $3)
        "#,
    )
    .bind(actor_staff_user_id)
    .bind(variant_id.to_string())
    .bind(reason)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Availability {
        variant_id,
        available_quantity: resulting_available,
        reserved_quantity: current.reserved_quantity,
        committed_quantity: current.committed_quantity,
        low_stock_threshold: resulting_threshold,
        low_stock: resulting_available <= resulting_threshold,
    })
}

pub async fn reserve(
    pool: &PgPool,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
) -> Result<Availability, InventoryOperationError> {
    transition(pool, variant_id, quantity, reason, StockTransition::Reserve).await
}

pub async fn reserve_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
) -> Result<Availability, InventoryOperationError> {
    transition_in_transaction(
        transaction,
        variant_id,
        quantity,
        reason,
        StockTransition::Reserve,
    )
    .await
}

pub async fn release(
    pool: &PgPool,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
) -> Result<Availability, InventoryOperationError> {
    transition(pool, variant_id, quantity, reason, StockTransition::Release).await
}

pub async fn release_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
) -> Result<Availability, InventoryOperationError> {
    transition_in_transaction(
        transaction,
        variant_id,
        quantity,
        reason,
        StockTransition::Release,
    )
    .await
}

pub async fn commit(
    pool: &PgPool,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
) -> Result<Availability, InventoryOperationError> {
    transition(pool, variant_id, quantity, reason, StockTransition::Commit).await
}

pub async fn commit_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
) -> Result<Availability, InventoryOperationError> {
    transition_in_transaction(
        transaction,
        variant_id,
        quantity,
        reason,
        StockTransition::Commit,
    )
    .await
}

pub async fn restock_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
) -> Result<Availability, InventoryOperationError> {
    transition_in_transaction(
        transaction,
        variant_id,
        quantity,
        reason,
        StockTransition::Restock,
    )
    .await
}

async fn transition(
    pool: &PgPool,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
    operation: StockTransition,
) -> Result<Availability, InventoryOperationError> {
    let mut transaction = pool.begin().await?;
    let availability =
        transition_in_transaction(&mut transaction, variant_id, quantity, reason, operation)
            .await?;
    transaction.commit().await?;
    Ok(availability)
}

async fn transition_in_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    variant_id: Uuid,
    quantity: i64,
    reason: &str,
    operation: StockTransition,
) -> Result<Availability, InventoryOperationError> {
    if quantity <= 0 {
        return Err(InventoryOperationError::InvalidQuantity);
    }
    let reason = validate_reason(reason)?;
    let current = locked_quantities(transaction, variant_id).await?;

    let (available_quantity, reserved_quantity, committed_quantity, quantity_delta) =
        match operation {
            StockTransition::Reserve => {
                let available = current
                    .available_quantity
                    .checked_sub(quantity)
                    .filter(|value| *value >= 0)
                    .ok_or(InventoryOperationError::InsufficientAvailable)?;
                let reserved = current
                    .reserved_quantity
                    .checked_add(quantity)
                    .ok_or(InventoryOperationError::QuantityOverflow)?;
                (available, reserved, current.committed_quantity, -quantity)
            }
            StockTransition::Release => {
                let reserved = current
                    .reserved_quantity
                    .checked_sub(quantity)
                    .filter(|value| *value >= 0)
                    .ok_or(InventoryOperationError::InsufficientReserved)?;
                let available = current
                    .available_quantity
                    .checked_add(quantity)
                    .ok_or(InventoryOperationError::QuantityOverflow)?;
                (available, reserved, current.committed_quantity, quantity)
            }
            StockTransition::Commit => {
                let reserved = current
                    .reserved_quantity
                    .checked_sub(quantity)
                    .filter(|value| *value >= 0)
                    .ok_or(InventoryOperationError::InsufficientReserved)?;
                let committed = current
                    .committed_quantity
                    .checked_add(quantity)
                    .ok_or(InventoryOperationError::QuantityOverflow)?;
                (current.available_quantity, reserved, committed, -quantity)
            }
            StockTransition::Restock => {
                let committed = current
                    .committed_quantity
                    .checked_sub(quantity)
                    .filter(|value| *value >= 0)
                    .ok_or(InventoryOperationError::InsufficientAvailable)?;
                let available = current
                    .available_quantity
                    .checked_add(quantity)
                    .ok_or(InventoryOperationError::QuantityOverflow)?;
                (available, current.reserved_quantity, committed, quantity)
            }
        };

    sqlx::query(
        r#"
        UPDATE inventory_items
        SET available_quantity = $2,
            reserved_quantity = $3,
            committed_quantity = $4,
            updated_at = now()
        WHERE variant_id = $1
        "#,
    )
    .bind(variant_id)
    .bind(available_quantity)
    .bind(reserved_quantity)
    .bind(committed_quantity)
    .execute(&mut **transaction)
    .await?;
    insert_movement(
        transaction,
        variant_id,
        None,
        operation.movement_type(),
        quantity_delta,
        available_quantity,
        reason,
    )
    .await?;
    Ok(Availability {
        variant_id,
        available_quantity,
        reserved_quantity,
        committed_quantity,
        low_stock_threshold: current.low_stock_threshold,
        low_stock: available_quantity <= current.low_stock_threshold,
    })
}

async fn locked_quantities(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    variant_id: Uuid,
) -> Result<InventoryQuantities, InventoryOperationError> {
    sqlx::query_as(
        r#"
        SELECT available_quantity, reserved_quantity, committed_quantity, low_stock_threshold
        FROM inventory_items
        WHERE variant_id = $1
        FOR UPDATE
        "#,
    )
    .bind(variant_id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or(InventoryOperationError::NotFound)
}

#[allow(clippy::too_many_arguments)]
async fn insert_movement(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    variant_id: Uuid,
    actor_staff_user_id: Option<Uuid>,
    movement_type: &str,
    quantity_delta: i64,
    resulting_available_quantity: i64,
    reason: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO inventory_movements (
            id, variant_id, actor_staff_user_id, movement_type,
            quantity_delta, resulting_available_quantity, reason
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(variant_id)
    .bind(actor_staff_user_id)
    .bind(movement_type)
    .bind(quantity_delta)
    .bind(resulting_available_quantity)
    .bind(reason)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn validate_reason(reason: &str) -> Result<&str, InventoryOperationError> {
    let reason = reason.trim();
    if !(3..=500).contains(&reason.len()) {
        return Err(InventoryOperationError::InvalidReason);
    }
    Ok(reason)
}

fn validate_adjustment(
    quantity_delta: i64,
    low_stock_threshold: Option<i64>,
    reason: &str,
) -> Result<&str, InventoryOperationError> {
    if quantity_delta == 0 {
        return Err(InventoryOperationError::InvalidQuantity);
    }
    if low_stock_threshold.is_some_and(|value| value < 0) {
        return Err(InventoryOperationError::InvalidThreshold);
    }
    validate_reason(reason)
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
