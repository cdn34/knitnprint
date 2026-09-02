use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    cancellations::{OrderOperations, Refund, load_operations, load_refunds},
    discounts::{
        EvaluationError as DiscountEvaluationError, OrderDiscount, evaluate_in_transaction,
        load_order_discount, record_usage,
    },
    error::ErrorBody,
    fulfillment::{Fulfillment, load_for_order as load_fulfillments},
    inventory::{InventoryOperationError, commit_in_transaction, reserve_in_transaction},
    notifications::{
        NotificationStatus, enqueue_order_confirmation, load_for_order as load_notifications,
    },
    packlink::{PackageItem, PacklinkPackage, PacklinkService, packages_for_items},
    payments::{PaymentAttempt, PaymentStatusEvent, load_attempts, load_status_events},
    settings::{
        OrderShipping, OrderTax, PricingError, evaluate_in_transaction as evaluate_commercial,
        evaluate_packlink_in_transaction, load_order_shipping, load_order_tax,
        record_order_snapshots,
    },
};

const CART_COOKIE: &str = "knitprint_cart";
const ORDERS_READ: &str = "orders.read";
const ORDERS_FULFILL: &str = "orders.fulfill";

#[derive(Deserialize, ToSchema)]
pub struct CreateOrderRequest {
    pub payment_method: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ManualPaymentRequest {
    pub reason: String,
}

#[derive(Serialize, ToSchema)]
pub struct Order {
    pub id: Uuid,
    pub order_number: String,
    pub order_status: String,
    pub payment_status: String,
    pub fulfillment_status: String,
    pub currency: String,
    pub subtotal_minor: i64,
    pub discount_minor: i64,
    pub discount: Option<OrderDiscount>,
    pub shipping_minor: i64,
    pub shipping: OrderShipping,
    pub tax_minor: i64,
    pub tax: OrderTax,
    pub total_minor: i64,
    pub customer: OrderCustomer,
    pub shipping_address: OrderAddress,
    pub lines: Vec<OrderLine>,
    pub payment: OrderPayment,
    pub refunds: Vec<Refund>,
    pub operations: OrderOperations,
    pub fulfillments: Vec<Fulfillment>,
    pub notifications: Vec<NotificationStatus>,
    pub timeline: Vec<OrderEvent>,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct OrderCustomer {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
}

#[derive(Serialize, ToSchema)]
pub struct OrderAddress {
    pub recipient_name: String,
    pub line1: String,
    pub line2: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country_code: String,
    pub phone: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct OrderLine {
    pub id: Uuid,
    pub product_title: String,
    pub variant_title: String,
    pub sku: String,
    pub quantity: i32,
    pub fulfilled_quantity: i64,
    pub unit_price_minor: i64,
    pub line_total_minor: i64,
    pub currency: String,
    pub customization: Option<Value>,
    pub customization_media_asset_id: Option<Uuid>,
    pub customization_media_asset_ids: Vec<Uuid>,
    pub personalization_context: Option<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderPayment {
    pub provider: String,
    pub status: String,
    pub amount_minor: i64,
    pub currency: String,
    pub paid_at: Option<String>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub attempts: Vec<PaymentAttempt>,
    pub history: Vec<PaymentStatusEvent>,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct OrderEvent {
    pub id: Uuid,
    pub event_type: String,
    pub title: String,
    pub detail: String,
    pub actor_display_name: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct OrderSummary {
    pub id: Uuid,
    pub order_number: String,
    pub customer_name: String,
    pub customer_email: String,
    pub order_status: String,
    pub payment_status: String,
    pub fulfillment_status: String,
    pub item_count: i64,
    pub total_minor: i64,
    pub currency: String,
    pub created_at: String,
}

#[derive(FromRow)]
struct CheckoutCart {
    id: Uuid,
    status: String,
    currency: Option<String>,
    customer_id: Option<Uuid>,
    shipping_address_id: Option<Uuid>,
    discount_id: Option<Uuid>,
    shipping_method_id: Option<Uuid>,
    shipping_quote_id: Option<Uuid>,
    expired: bool,
}

#[derive(FromRow)]
struct CheckoutLine {
    product_id: Uuid,
    variant_id: Uuid,
    product_title: String,
    product_status: String,
    variant_title: String,
    sku: String,
    quantity: i32,
    stored_price_minor: i64,
    current_price_minor: i64,
    stored_currency: String,
    current_currency: String,
    shipping_weight_grams: i32,
    shipping_width_cm: i32,
    shipping_length_cm: i32,
    shipping_height_cm: i32,
    shipping_empty_weight_grams: i32,
    shipping_units_per_package: i32,
    shipping_profile_configured: bool,
    customization: Option<Value>,
    customization_media_asset_id: Option<Uuid>,
    customization_media_asset_ids: Vec<Uuid>,
    personalization_context: Option<Value>,
}

#[derive(FromRow)]
struct DeliverySnapshot {
    customer_id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    customer_phone: String,
    recipient_name: String,
    line1: String,
    line2: String,
    city: String,
    region: String,
    postal_code: String,
    country_code: String,
    address_phone: String,
}

#[derive(FromRow)]
struct OrderHead {
    id: Uuid,
    order_number: String,
    order_status: String,
    payment_status: String,
    fulfillment_status: String,
    currency: String,
    subtotal_minor: i64,
    discount_minor: i64,
    shipping_minor: i64,
    tax_minor: i64,
    total_minor: i64,
    customer_email: String,
    customer_first_name: String,
    customer_last_name: String,
    customer_phone: String,
    shipping_recipient_name: String,
    shipping_line1: String,
    shipping_line2: String,
    shipping_city: String,
    shipping_region: String,
    shipping_postal_code: String,
    shipping_country_code: String,
    shipping_phone: String,
    payment_id: Uuid,
    payment_provider: String,
    payment_amount_minor: i64,
    payment_currency: String,
    paid_at: Option<String>,
    payment_failure_code: Option<String>,
    payment_failure_message: Option<String>,
    created_at: String,
}

#[utoipa::path(
    post,
    path = "/api/orders",
    tag = "orders",
    params(("Idempotency-Key" = String, Header)),
    request_body = CreateOrderRequest,
    responses(
        (status = 200, body = Order, description = "Idempotent replay"),
        (status = 201, body = Order),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<CreateOrderRequest>,
) -> Response {
    let provider = match input.payment_method.as_str() {
        "manual" if state.manual_payments_enabled => "manual",
        "stripe" if state.payments.enabled() => "stripe",
        _ => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "payment_method_unavailable",
                "That payment method is not available.",
            );
        }
    };
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return invalid_idempotency_key();
    };
    let Some(cookie) = jar.get(CART_COOKIE) else {
        return cart_not_ready();
    };
    let token_hash: [u8; 32] = Sha256::digest(cookie.value().as_bytes()).into();
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    match create_order(
        pool,
        token_hash,
        idempotency_hash,
        provider,
        &state.packlink,
    )
    .await
    {
        Ok((order, created)) => no_store(
            (
                if created {
                    StatusCode::CREATED
                } else {
                    StatusCode::OK
                },
                Json(order),
            )
                .into_response(),
        ),
        Err(CreateError::NotReady) => cart_not_ready(),
        Err(CreateError::Changed) => cart_changed(),
        Err(CreateError::IdempotencyConflict) => error(
            StatusCode::CONFLICT,
            "checkout_idempotency_conflict",
            "That checkout key was already used for a different request.",
        ),
        Err(CreateError::Database) => unavailable(),
    }
}

#[utoipa::path(
    get,
    path = "/api/orders/{order_id}",
    tag = "orders",
    params(("order_id" = Uuid, Path)),
    responses(
        (status = 200, body = Order),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn customer_detail(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(order_id): Path<Uuid>,
) -> Response {
    let Some(cookie) = jar.get(CART_COOKIE) else {
        return not_found();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let token_hash: [u8; 32] = Sha256::digest(cookie.value().as_bytes()).into();
    let owned = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM orders order_record
            JOIN carts cart ON cart.id = order_record.cart_id
            WHERE order_record.id = $1 AND cart.token_hash = $2
        )
        "#,
    )
    .bind(order_id)
    .bind(token_hash.as_slice())
    .fetch_one(&pool)
    .await;
    match owned {
        Ok(true) => match load_order(&pool, order_id).await {
            Ok(Some(mut order)) => {
                for refund in &mut order.refunds {
                    refund.internal_note = None;
                }
                no_store(Json(order).into_response())
            }
            Ok(None) => not_found(),
            Err(_) => unavailable(),
        },
        Ok(false) => not_found(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/orders",
    tag = "admin orders",
    responses(
        (status = 200, body = [OrderSummary]),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn admin_list(State(state): State<AppState>, actor: AuthenticatedStaff) -> Response {
    if let Err(response) = require_capability(&actor, ORDERS_READ) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match sqlx::query_as::<_, OrderSummary>(
        r#"
        SELECT order_record.id, order_record.order_number,
               btrim(order_record.customer_first_name || ' ' || order_record.customer_last_name) AS customer_name,
               order_record.customer_email, order_record.order_status,
               order_record.payment_status, order_record.fulfillment_status,
               COALESCE(sum(line.quantity), 0)::bigint AS item_count,
               order_record.total_minor, order_record.currency::text AS currency,
               to_char(order_record.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM orders order_record
        LEFT JOIN order_lines line ON line.order_id = order_record.id
        GROUP BY order_record.id
        ORDER BY order_record.created_at DESC, order_record.id DESC
        LIMIT 200
        "#,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(orders) => no_store(Json(orders).into_response()),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/orders/{order_id}",
    tag = "admin orders",
    params(("order_id" = Uuid, Path)),
    responses(
        (status = 200, body = Order),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody)
    )
)]
pub async fn admin_detail(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(order_id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_capability(&actor, ORDERS_READ) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match load_order(&pool, order_id).await {
        Ok(Some(order)) => no_store(Json(order).into_response()),
        Ok(None) => not_found(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/orders/{order_id}/manual-payment",
    tag = "admin orders",
    params(("order_id" = Uuid, Path)),
    request_body = ManualPaymentRequest,
    responses(
        (status = 200, body = Order),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody)
    )
)]
pub async fn record_manual_payment(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(order_id): Path<Uuid>,
    Json(input): Json<ManualPaymentRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, ORDERS_FULFILL) {
        return response.into_response();
    }
    let reason = input.reason.trim();
    if !(3..=500).contains(&reason.len()) {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_payment_reason",
            "Provide a payment reason of 3–500 characters.",
        );
    }
    if !state.manual_payments_enabled {
        return error(
            StatusCode::FORBIDDEN,
            "manual_payment_disabled",
            "Manual payment recording is disabled in this environment.",
        );
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match pay_order(&pool, order_id, actor.id, reason).await {
        Ok(()) => match load_order(&pool, order_id).await {
            Ok(Some(order)) => no_store(Json(order).into_response()),
            _ => unavailable(),
        },
        Err(PaymentError::NotFound) => not_found(),
        Err(PaymentError::Conflict) => error(
            StatusCode::CONFLICT,
            "payment_state_conflict",
            "This order cannot accept a manual payment in its current state.",
        ),
        Err(PaymentError::Database) => unavailable(),
    }
}

enum CreateError {
    NotReady,
    Changed,
    IdempotencyConflict,
    Database,
}

async fn create_order(
    pool: &PgPool,
    token_hash: [u8; 32],
    idempotency_hash: [u8; 32],
    provider: &str,
    packlink: &PacklinkService,
) -> Result<(Order, bool), CreateError> {
    let mut transaction = pool.begin().await.map_err(|_| CreateError::Database)?;
    let cart = sqlx::query_as::<_, CheckoutCart>(
        r#"
        SELECT id, status, currency::text AS currency, customer_id, shipping_address_id, discount_id,
               shipping_method_id, shipping_quote_id,
               expires_at <= now() AS expired
        FROM carts WHERE token_hash = $1
        FOR UPDATE
        "#,
    )
    .bind(token_hash.as_slice())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?
    .ok_or(CreateError::NotReady)?;

    if cart.status == "converted" {
        let existing: (Uuid, Vec<u8>, String) = sqlx::query_as(
            r#"
            SELECT order_record.id, order_record.checkout_idempotency_hash, payment.provider
            FROM orders order_record
            JOIN order_payments payment ON payment.order_id = order_record.id
            WHERE order_record.cart_id = $1
            "#,
        )
        .bind(cart.id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| CreateError::Database)?;
        if existing.1.as_slice() != idempotency_hash.as_slice() || existing.2 != provider {
            return Err(CreateError::IdempotencyConflict);
        }
        transaction
            .commit()
            .await
            .map_err(|_| CreateError::Database)?;
        return load_order(pool, existing.0)
            .await
            .map_err(|_| CreateError::Database)?
            .map(|order| (order, false))
            .ok_or(CreateError::Database);
    }
    if cart.status != "active"
        || cart.expired
        || cart.currency.is_none()
        || cart.customer_id.is_none()
        || cart.shipping_address_id.is_none()
    {
        return Err(CreateError::NotReady);
    }

    let lines = sqlx::query_as::<_, CheckoutLine>(
        r#"
        SELECT product.id AS product_id, variant.id AS variant_id,
               product.title AS product_title, product.status AS product_status,
               variant.title AS variant_title, variant.sku, line.quantity,
               line.unit_price_minor AS stored_price_minor,
               variant.price_minor AS current_price_minor,
               line.currency::text AS stored_currency,
               variant.currency::text AS current_currency,
               product.shipping_weight_grams,
               COALESCE(package_profile.width_cm, product.shipping_width_cm) AS shipping_width_cm,
               COALESCE(package_profile.length_cm, product.shipping_length_cm) AS shipping_length_cm,
               COALESCE(package_profile.height_cm, product.shipping_height_cm) AS shipping_height_cm,
               COALESCE(package_profile.empty_weight_grams, 0) AS shipping_empty_weight_grams,
               product.shipping_units_per_package,
               (product.shipping_profile_configured AND COALESCE(package_profile.active, false)) AS shipping_profile_configured,
               line.customization, line.customization_media_asset_id,
               line.customization_media_asset_ids,
               CASE WHEN line.customization IS NULL THEN NULL ELSE
                 jsonb_build_object(
                   'version', 1,
                   'views', COALESCE(personalization.views, '[]'::jsonb)
                 )
               END AS personalization_context
        FROM cart_lines line
        JOIN product_variants variant ON variant.id = line.variant_id
        JOIN products product ON product.id = variant.product_id
        JOIN inventory_items inventory ON inventory.variant_id = variant.id
        LEFT JOIN shipping_package_profiles package_profile
          ON package_profile.id=product.shipping_package_profile_id
        LEFT JOIN product_personalization personalization ON personalization.product_id = product.id
        WHERE line.cart_id = $1
        ORDER BY variant.id
        FOR UPDATE OF variant, inventory
        "#,
    )
    .bind(cart.id)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    if lines.is_empty() {
        return Err(CreateError::NotReady);
    }
    let currency = cart.currency.as_deref().ok_or(CreateError::NotReady)?;
    let mut subtotal_minor = 0_i64;
    for line in &lines {
        if line.product_status != "active"
            || line.stored_price_minor != line.current_price_minor
            || line.stored_currency != line.current_currency
            || line.current_currency != currency
        {
            return Err(CreateError::Changed);
        }
        subtotal_minor = subtotal_minor
            .checked_add(
                line.current_price_minor
                    .checked_mul(i64::from(line.quantity))
                    .ok_or(CreateError::Changed)?,
            )
            .ok_or(CreateError::Changed)?;
    }
    let delivery = delivery_snapshot(
        &mut transaction,
        cart.id,
        cart.customer_id.ok_or(CreateError::NotReady)?,
        cart.shipping_address_id.ok_or(CreateError::NotReady)?,
    )
    .await?
    .ok_or(CreateError::NotReady)?;

    let evaluated_discount = if let Some(discount_id) = cart.discount_id {
        Some(
            evaluate_in_transaction(
                &mut transaction,
                discount_id,
                subtotal_minor,
                currency,
                Some(delivery.customer_id),
            )
            .await
            .map_err(|error| match error {
                DiscountEvaluationError::Unavailable => CreateError::Changed,
                DiscountEvaluationError::Database(_) => CreateError::Database,
            })?,
        )
    } else {
        None
    };
    let discount_minor = evaluated_discount
        .as_ref()
        .map_or(0, |discount| discount.amount_minor);
    let merchandise_minor = subtotal_minor
        .checked_sub(discount_minor)
        .ok_or(CreateError::Changed)?;
    let commercial_pricing = if packlink.enabled() {
        let packages = checkout_packages(&lines).ok_or(CreateError::Changed)?;
        let request_hash =
            packlink.request_hash(&delivery.country_code, &delivery.postal_code, &packages);
        evaluate_packlink_in_transaction(
            &mut transaction,
            cart.id,
            currency,
            &delivery.country_code,
            cart.shipping_quote_id,
            merchandise_minor,
            Some(request_hash.as_slice()),
        )
        .await
    } else {
        evaluate_commercial(
            &mut transaction,
            currency,
            &delivery.country_code,
            cart.shipping_method_id,
            merchandise_minor,
        )
        .await
    }
    .map_err(|error| match error {
        PricingError::Unavailable => CreateError::Changed,
        PricingError::Database(_) => CreateError::Database,
    })?;
    let shipping_minor = commercial_pricing.shipping.amount_minor;
    let tax_minor = commercial_pricing.tax.amount_minor;
    let total_minor = subtotal_minor
        .checked_sub(discount_minor)
        .and_then(|total| total.checked_add(shipping_minor))
        .and_then(|total| total.checked_add(tax_minor))
        .ok_or(CreateError::Changed)?;

    let order_id = Uuid::now_v7();
    let order_number: String = sqlx::query_scalar(
        r#"
        INSERT INTO orders (
            id, order_number, cart_id, customer_id, checkout_idempotency_hash,
            currency, subtotal_minor, discount_minor, shipping_minor, tax_minor, total_minor, customer_email,
            customer_first_name, customer_last_name, customer_phone,
            shipping_recipient_name, shipping_line1, shipping_line2, shipping_city,
            shipping_region, shipping_postal_code, shipping_country_code, shipping_phone
        ) VALUES (
            $1, 'KP-' || to_char(now(), 'YYYY') || '-' || lpad(nextval('order_number_sequence')::text, 6, '0'),
            $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22
        ) RETURNING order_number
        "#,
    )
    .bind(order_id)
    .bind(cart.id)
    .bind(delivery.customer_id)
    .bind(idempotency_hash.as_slice())
    .bind(currency)
    .bind(subtotal_minor)
    .bind(discount_minor)
    .bind(shipping_minor)
    .bind(tax_minor)
    .bind(total_minor)
    .bind(&delivery.email)
    .bind(&delivery.first_name)
    .bind(&delivery.last_name)
    .bind(&delivery.customer_phone)
    .bind(&delivery.recipient_name)
    .bind(&delivery.line1)
    .bind(&delivery.line2)
    .bind(&delivery.city)
    .bind(&delivery.region)
    .bind(&delivery.postal_code)
    .bind(&delivery.country_code)
    .bind(&delivery.address_phone)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;

    if let Some(discount) = &evaluated_discount {
        record_usage(&mut transaction, discount, order_id, delivery.customer_id)
            .await
            .map_err(|_| CreateError::Database)?;
    }
    record_order_snapshots(&mut transaction, order_id, &commercial_pricing)
        .await
        .map_err(|_| CreateError::Database)?;

    for (position, line) in lines.iter().enumerate() {
        let order_line_id = Uuid::now_v7();
        sqlx::query(
            r#"
            INSERT INTO order_lines (
                id, order_id, product_id, variant_id, product_title, variant_title,
                sku, quantity, unit_price_minor, line_total_minor, currency, position,
                customization, customization_media_asset_id, customization_media_asset_ids,
                personalization_context
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9 * $8, $10, $11, $12, $13, $14, $15)
            "#,
        )
        .bind(order_line_id)
        .bind(order_id)
        .bind(line.product_id)
        .bind(line.variant_id)
        .bind(&line.product_title)
        .bind(&line.variant_title)
        .bind(&line.sku)
        .bind(line.quantity)
        .bind(line.current_price_minor)
        .bind(&line.current_currency)
        .bind(position as i32)
        .bind(&line.customization)
        .bind(line.customization_media_asset_id)
        .bind(&line.customization_media_asset_ids)
        .bind(&line.personalization_context)
        .execute(&mut *transaction)
        .await
        .map_err(|_| CreateError::Database)?;
        for (media_position, media_id) in line.customization_media_asset_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO order_line_personalization_media (order_line_id, media_asset_id, position) VALUES ($1, $2, $3)",
            )
            .bind(order_line_id)
            .bind(media_id)
            .bind(media_position as i32)
            .execute(&mut *transaction)
            .await
            .map_err(|_| CreateError::Database)?;
        }
        reserve_in_transaction(
            &mut transaction,
            line.variant_id,
            i64::from(line.quantity),
            &format!("Reserved for order {order_number}"),
        )
        .await
        .map_err(map_inventory_create_error)?;
    }
    sqlx::query(
        "INSERT INTO order_payments (id, order_id, provider, amount_minor, currency) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(order_id)
    .bind(provider)
    .bind(total_minor)
    .bind(currency)
    .execute(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    insert_event(
        &mut transaction,
        order_id,
        None,
        Some(delivery.customer_id),
        "order.created",
        "Order created",
        "Stock reserved and payment pending.",
    )
    .await
    .map_err(|_| CreateError::Database)?;
    sqlx::query(
        "INSERT INTO audit_log (actor_customer_id, action, entity_type, entity_id) VALUES ($1, 'order.create', 'order', $2)",
    )
    .bind(delivery.customer_id)
    .bind(order_id.to_string())
    .execute(&mut *transaction)
    .await
    .map_err(|_| CreateError::Database)?;
    sqlx::query("UPDATE carts SET status = 'converted', updated_at = now() WHERE id = $1")
        .bind(cart.id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| CreateError::Database)?;
    transaction
        .commit()
        .await
        .map_err(|_| CreateError::Database)?;
    let order = load_order(pool, order_id)
        .await
        .map_err(|_| CreateError::Database)?
        .ok_or(CreateError::Database)?;
    Ok((order, true))
}

fn checkout_packages(lines: &[CheckoutLine]) -> Option<Vec<PacklinkPackage>> {
    if lines.iter().any(|line| !line.shipping_profile_configured) {
        return None;
    }
    let mut items = BTreeMap::<Uuid, PackageItem>::new();
    for line in lines {
        let item = items.entry(line.product_id).or_insert_with(|| PackageItem {
            quantity: 0,
            unit_weight_grams: line.shipping_weight_grams,
            width_cm: line.shipping_width_cm,
            length_cm: line.shipping_length_cm,
            height_cm: line.shipping_height_cm,
            empty_weight_grams: line.shipping_empty_weight_grams,
            units_per_package: line.shipping_units_per_package,
        });
        item.quantity = item.quantity.checked_add(i64::from(line.quantity))?;
    }
    packages_for_items(&items.into_values().collect::<Vec<_>>())
}

async fn delivery_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
    customer_id: Uuid,
    address_id: Uuid,
) -> Result<Option<DeliverySnapshot>, CreateError> {
    sqlx::query_as(
        r#"
        SELECT customer.id AS customer_id, customer.email::text AS email,
               customer.first_name, customer.last_name, customer.phone AS customer_phone,
               address.recipient_name, address.line1, address.line2, address.city,
               address.region, address.postal_code, address.country_code::text AS country_code,
               address.phone AS address_phone
        FROM carts cart
        JOIN customers customer ON customer.id = cart.customer_id
        JOIN customer_addresses address ON address.id = cart.shipping_address_id
          AND address.customer_id = customer.id
        WHERE cart.id = $1 AND customer.id = $2 AND address.id = $3
          AND customer.anonymized_at IS NULL AND customer.retention_expires_at > now()
        "#,
    )
    .bind(cart_id)
    .bind(customer_id)
    .bind(address_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| CreateError::Database)
}

enum PaymentError {
    NotFound,
    Conflict,
    Database,
}

async fn pay_order(
    pool: &PgPool,
    order_id: Uuid,
    actor_id: Uuid,
    reason: &str,
) -> Result<(), PaymentError> {
    let mut transaction = pool.begin().await.map_err(|_| PaymentError::Database)?;
    let status: Option<(String, String, String)> = sqlx::query_as(
        r#"
        SELECT order_record.order_status, payment.status, payment.provider
        FROM orders order_record JOIN order_payments payment ON payment.order_id = order_record.id
        WHERE order_record.id = $1 FOR UPDATE OF order_record, payment
        "#,
    )
    .bind(order_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| PaymentError::Database)?;
    let Some((order_status, payment_status, provider)) = status else {
        return Err(PaymentError::NotFound);
    };
    if order_status == "confirmed" && payment_status == "paid" {
        enqueue_order_confirmation(&mut transaction, order_id)
            .await
            .map_err(|_| PaymentError::Database)?;
        transaction
            .commit()
            .await
            .map_err(|_| PaymentError::Database)?;
        return Ok(());
    }
    if order_status != "pending" || payment_status != "pending" || provider != "manual" {
        return Err(PaymentError::Conflict);
    }
    let lines: Vec<(Uuid, i32)> = sqlx::query_as(
        "SELECT variant_id, quantity FROM order_lines WHERE order_id = $1 ORDER BY variant_id",
    )
    .bind(order_id)
    .fetch_all(&mut *transaction)
    .await
    .map_err(|_| PaymentError::Database)?;
    for (variant_id, quantity) in lines {
        commit_in_transaction(
            &mut transaction,
            variant_id,
            i64::from(quantity),
            &format!("Committed for paid order {order_id}"),
        )
        .await
        .map_err(|_| PaymentError::Database)?;
    }
    sqlx::query(
        "UPDATE order_payments SET status = 'paid', paid_at = now(), updated_at = now() WHERE order_id = $1",
    )
    .bind(order_id)
    .execute(&mut *transaction)
    .await
    .map_err(|_| PaymentError::Database)?;
    let payment_id: Uuid = sqlx::query_scalar("SELECT id FROM order_payments WHERE order_id = $1")
        .bind(order_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|_| PaymentError::Database)?;
    sqlx::query(
        r#"
        INSERT INTO payment_status_events (
            id, order_payment_id, provider, event_type, provider_status, detail
        ) VALUES ($1, $2, 'manual', 'payment.manual_recorded', 'succeeded', $3)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(payment_id)
    .bind(reason)
    .execute(&mut *transaction)
    .await
    .map_err(|_| PaymentError::Database)?;
    sqlx::query(
        "UPDATE orders SET order_status = 'confirmed', payment_status = 'paid', updated_at = now() WHERE id = $1",
    )
    .bind(order_id)
    .execute(&mut *transaction)
    .await
    .map_err(|_| PaymentError::Database)?;
    insert_event(
        &mut transaction,
        order_id,
        Some(actor_id),
        None,
        "payment.paid",
        "Manual payment recorded",
        reason,
    )
    .await
    .map_err(|_| PaymentError::Database)?;
    sqlx::query(
        "INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason) VALUES ($1, 'order.manual_payment', 'order', $2, $3)",
    )
    .bind(actor_id)
    .bind(order_id.to_string())
    .bind(reason)
    .execute(&mut *transaction)
    .await
    .map_err(|_| PaymentError::Database)?;
    enqueue_order_confirmation(&mut transaction, order_id)
        .await
        .map_err(|_| PaymentError::Database)?;
    transaction
        .commit()
        .await
        .map_err(|_| PaymentError::Database)
}

#[allow(clippy::too_many_arguments)]
async fn insert_event(
    transaction: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    actor_staff_user_id: Option<Uuid>,
    actor_customer_id: Option<Uuid>,
    event_type: &str,
    title: &str,
    detail: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO order_events (
            id, order_id, actor_staff_user_id, actor_customer_id, event_type, title, detail
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(order_id)
    .bind(actor_staff_user_id)
    .bind(actor_customer_id)
    .bind(event_type)
    .bind(title)
    .bind(detail)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub(crate) async fn load_order(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Option<Order>, sqlx::Error> {
    let Some(head) = sqlx::query_as::<_, OrderHead>(
        r#"
        SELECT order_record.id, order_record.order_number, order_record.order_status,
               order_record.payment_status, order_record.fulfillment_status,
               order_record.currency::text AS currency, order_record.subtotal_minor,
               order_record.discount_minor, order_record.shipping_minor, order_record.tax_minor,
               order_record.total_minor, order_record.customer_email,
               order_record.customer_first_name, order_record.customer_last_name,
               order_record.customer_phone, order_record.shipping_recipient_name,
               order_record.shipping_line1, order_record.shipping_line2, order_record.shipping_city,
               order_record.shipping_region, order_record.shipping_postal_code,
               order_record.shipping_country_code::text AS shipping_country_code,
               order_record.shipping_phone, payment.id AS payment_id,
               payment.provider AS payment_provider,
               payment.amount_minor AS payment_amount_minor,
               payment.currency::text AS payment_currency,
               payment.failure_code AS payment_failure_code,
               payment.failure_message AS payment_failure_message,
               CASE WHEN payment.paid_at IS NULL THEN NULL ELSE
                 to_char(payment.paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') END AS paid_at,
               to_char(order_record.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM orders order_record JOIN order_payments payment ON payment.order_id = order_record.id
        WHERE order_record.id = $1
        "#,
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let lines = sqlx::query_as::<_, OrderLine>(
        r#"
        SELECT line.id, line.product_title, line.variant_title, line.sku, line.quantity,
               COALESCE((SELECT sum(fulfilled.quantity)::bigint FROM fulfillment_lines fulfilled WHERE fulfilled.order_line_id = line.id), 0) AS fulfilled_quantity,
               line.unit_price_minor, line.line_total_minor, line.currency::text AS currency,
               line.customization, line.customization_media_asset_id,
               line.customization_media_asset_ids, line.personalization_context
        FROM order_lines line WHERE line.order_id = $1 ORDER BY line.position, line.id
        "#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    let attempts = load_attempts(pool, head.payment_id).await?;
    let history = load_status_events(pool, head.payment_id).await?;
    let refunds = load_refunds(pool, order_id).await?;
    let discount = load_order_discount(pool, order_id).await?;
    let operations = load_operations(
        pool,
        order_id,
        &head.order_status,
        &head.payment_status,
        &head.fulfillment_status,
        head.payment_amount_minor,
    )
    .await?;
    let fulfillments = load_fulfillments(pool, order_id).await?;
    let notifications = load_notifications(pool, order_id).await?;
    let shipping = load_order_shipping(pool, order_id).await?;
    let tax = load_order_tax(pool, order_id).await?;
    let timeline = sqlx::query_as::<_, OrderEvent>(
        r#"
        SELECT event.id, event.event_type, event.title, event.detail,
               staff.display_name AS actor_display_name,
               to_char(event.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM order_events event
        LEFT JOIN staff_users staff ON staff.id = event.actor_staff_user_id
        WHERE event.order_id = $1 ORDER BY event.created_at, event.id
        "#,
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    Ok(Some(Order {
        id: head.id,
        order_number: head.order_number,
        order_status: head.order_status,
        payment_status: head.payment_status.clone(),
        fulfillment_status: head.fulfillment_status,
        currency: head.currency,
        subtotal_minor: head.subtotal_minor,
        discount_minor: head.discount_minor,
        discount,
        shipping_minor: head.shipping_minor,
        shipping,
        tax_minor: head.tax_minor,
        tax,
        total_minor: head.total_minor,
        customer: OrderCustomer {
            email: head.customer_email,
            first_name: head.customer_first_name,
            last_name: head.customer_last_name,
            phone: head.customer_phone,
        },
        shipping_address: OrderAddress {
            recipient_name: head.shipping_recipient_name,
            line1: head.shipping_line1,
            line2: head.shipping_line2,
            city: head.shipping_city,
            region: head.shipping_region,
            postal_code: head.shipping_postal_code,
            country_code: head.shipping_country_code,
            phone: head.shipping_phone,
        },
        lines,
        payment: OrderPayment {
            provider: head.payment_provider,
            status: head.payment_status,
            amount_minor: head.payment_amount_minor,
            currency: head.payment_currency,
            paid_at: head.paid_at,
            failure_code: head.payment_failure_code,
            failure_message: head.payment_failure_message,
            attempts,
            history,
        },
        refunds,
        operations,
        fulfillments,
        notifications,
        timeline,
        created_at: head.created_at,
    }))
}

fn map_inventory_create_error(error: InventoryOperationError) -> CreateError {
    let _ = error;
    CreateError::Database
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

fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    response
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}

fn invalid_idempotency_key() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_idempotency_key",
        "Provide a 16–128 character Idempotency-Key for checkout.",
    )
}

fn cart_not_ready() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "cart_not_ready",
        "Add available items and delivery details before creating an order.",
    )
}

fn cart_changed() -> Response {
    error(
        StatusCode::CONFLICT,
        "cart_changed",
        "The cart changed. Review it before creating the order.",
    )
}

fn not_found() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "order_not_found",
        "The order was not found.",
    )
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "orders_unavailable",
        "Orders are temporarily unavailable.",
    )
}
