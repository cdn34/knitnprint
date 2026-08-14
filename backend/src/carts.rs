use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use time::Duration;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    customer_auth::{CUSTOMER_SESSION_COOKIE, hash_token},
    customers::{CustomerAddressInput, GuestCustomerRequest, valid_guest},
    discounts::{
        AppliedDiscount, EvaluationError, evaluate, evaluate_in_transaction,
        find_by_code_in_transaction, normalize_code,
    },
    error::ErrorBody,
};

const CART_COOKIE: &str = "knitprint_cart";
const CART_DAYS: i64 = 30;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct AddCartItemRequest {
    pub variant_id: Uuid,
    pub quantity: i32,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateCartItemRequest {
    pub quantity: i32,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ApplyDiscountRequest {
    pub code: String,
}

#[derive(Serialize, ToSchema)]
pub struct Cart {
    pub id: Uuid,
    pub currency: Option<String>,
    pub items: Vec<CartItem>,
    pub item_count: i64,
    pub subtotal_minor: i64,
    pub discount: Option<AppliedDiscount>,
    pub discount_minor: i64,
    pub total_minor: i64,
    pub checkout_ready: bool,
    pub issues: Vec<CartIssue>,
    pub delivery: Option<CartDelivery>,
    pub expires_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct CartItem {
    pub id: Uuid,
    pub variant_id: Uuid,
    pub product_slug: String,
    pub product_title: String,
    pub variant_title: String,
    pub sku: String,
    pub quantity: i32,
    pub unit_price_minor: i64,
    pub currency: String,
    pub line_total_minor: i64,
    pub available_quantity: i64,
    pub available: bool,
    pub image_url: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CartIssue {
    pub code: String,
    pub line_id: Option<Uuid>,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct CartDelivery {
    pub customer_id: Uuid,
    pub address_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub address: CartAddress,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct CartAddress {
    pub recipient_name: String,
    pub line1: String,
    pub line2: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country_code: String,
    pub phone: String,
}

#[derive(FromRow)]
struct CartRow {
    currency: Option<String>,
    customer_id: Option<Uuid>,
    discount_id: Option<Uuid>,
    expires_at: String,
}

#[derive(FromRow)]
struct CartLineRow {
    id: Uuid,
    variant_id: Uuid,
    product_slug: String,
    product_title: String,
    product_status: String,
    variant_title: String,
    sku: String,
    quantity: i32,
    unit_price_minor: i64,
    currency: String,
    available_quantity: i64,
    image_url: Option<String>,
}

#[derive(FromRow)]
struct DeliveryRow {
    customer_id: Uuid,
    address_id: Uuid,
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
struct VariantForCart {
    price_minor: i64,
    currency: String,
    available_quantity: i64,
}

#[derive(FromRow)]
struct CartOwnership {
    customer_id: Option<Uuid>,
    shipping_address_id: Option<Uuid>,
    customer_type: Option<String>,
}

struct CartSession {
    id: Uuid,
    jar: CookieJar,
}

enum MutationClaim {
    New,
    Replay,
    Conflict,
}

pub async fn cleanup_expired(pool: &PgPool, batch_size: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        WITH claimed AS (
            SELECT id FROM carts
            WHERE status = 'expired' OR (status = 'active' AND expires_at <= now())
            ORDER BY expires_at, id
            FOR UPDATE SKIP LOCKED
            LIMIT $1
        )
        DELETE FROM carts cart USING claimed WHERE cart.id = claimed.id
        "#,
    )
    .bind(batch_size)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

#[utoipa::path(
    get,
    path = "/api/cart",
    tag = "cart",
    responses(
        (status = 200, body = Cart),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn get(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    let session = match resolve_cart(&pool, jar, state.secure_cookies).await {
        Ok(session) => session,
        Err(_) => return unavailable(),
    };
    cart_response(&pool, session).await
}

#[utoipa::path(
    post,
    path = "/api/cart/items",
    tag = "cart",
    params(("Idempotency-Key" = String, Header)),
    request_body = AddCartItemRequest,
    responses(
        (status = 200, body = Cart, description = "Idempotent replay"),
        (status = 201, body = Cart),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn add_item(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<AddCartItemRequest>,
) -> Response {
    if !(1..=99).contains(&input.quantity) {
        return invalid_quantity();
    }
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return invalid_idempotency_key();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let session = match resolve_cart(&pool, jar, state.secure_cookies).await {
        Ok(session) => session,
        Err(_) => return unavailable(),
    };
    let signature = format!("add:{}:{}", input.variant_id, input.quantity);
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    match claim_mutation(&mut transaction, session.id, &idempotency_hash, &signature).await {
        Ok(MutationClaim::Replay) => {
            if transaction.commit().await.is_err() {
                return unavailable();
            }
            return cart_response(&pool, session).await;
        }
        Ok(MutationClaim::Conflict) => return idempotency_conflict(),
        Ok(MutationClaim::New) => {}
        Err(_) => return unavailable(),
    }
    let variant = match active_variant(&mut transaction, input.variant_id).await {
        Ok(Some(variant)) => variant,
        Ok(None) => return unavailable_item(),
        Err(_) => return unavailable(),
    };
    let existing_quantity: i32 = match sqlx::query_scalar(
        "SELECT quantity FROM cart_lines WHERE cart_id = $1 AND variant_id = $2 FOR UPDATE",
    )
    .bind(session.id)
    .bind(input.variant_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(quantity) => quantity.unwrap_or(0),
        Err(_) => return unavailable(),
    };
    let Some(resulting_quantity) = existing_quantity.checked_add(input.quantity) else {
        return invalid_quantity();
    };
    if resulting_quantity > 99 || i64::from(resulting_quantity) > variant.available_quantity {
        return insufficient_stock();
    }
    if let Err(response) = ensure_currency(&mut transaction, session.id, &variant.currency).await {
        return response;
    }
    if sqlx::query(
        r#"
        INSERT INTO cart_lines (id, cart_id, variant_id, quantity, unit_price_minor, currency)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (cart_id, variant_id) DO UPDATE
        SET quantity = EXCLUDED.quantity,
            unit_price_minor = EXCLUDED.unit_price_minor,
            currency = EXCLUDED.currency,
            updated_at = now()
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(session.id)
    .bind(input.variant_id)
    .bind(resulting_quantity)
    .bind(variant.price_minor)
    .bind(&variant.currency)
    .execute(&mut *transaction)
    .await
    .is_err()
        || touch_cart(&mut transaction, session.id).await.is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    cart_response_with_status(&pool, session, StatusCode::CREATED).await
}

#[utoipa::path(
    patch,
    path = "/api/cart/items/{line_id}",
    tag = "cart",
    params(("line_id" = Uuid, Path), ("Idempotency-Key" = String, Header)),
    request_body = UpdateCartItemRequest,
    responses(
        (status = 200, body = Cart),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn update_item(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(line_id): Path<Uuid>,
    Json(input): Json<UpdateCartItemRequest>,
) -> Response {
    if !(1..=99).contains(&input.quantity) {
        return invalid_quantity();
    }
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return invalid_idempotency_key();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let session = match resolve_cart(&pool, jar, state.secure_cookies).await {
        Ok(session) => session,
        Err(_) => return unavailable(),
    };
    let signature = format!("update:{line_id}:{}", input.quantity);
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    match claim_mutation(&mut transaction, session.id, &idempotency_hash, &signature).await {
        Ok(MutationClaim::Replay) => {
            if transaction.commit().await.is_err() {
                return unavailable();
            }
            return cart_response(&pool, session).await;
        }
        Ok(MutationClaim::Conflict) => return idempotency_conflict(),
        Ok(MutationClaim::New) => {}
        Err(_) => return unavailable(),
    }
    let variant_id: Uuid = match sqlx::query_scalar(
        "SELECT variant_id FROM cart_lines WHERE id = $1 AND cart_id = $2 FOR UPDATE",
    )
    .bind(line_id)
    .bind(session.id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(variant_id)) => variant_id,
        Ok(None) => return line_not_found(),
        Err(_) => return unavailable(),
    };
    let variant = match active_variant(&mut transaction, variant_id).await {
        Ok(Some(variant)) => variant,
        Ok(None) => return unavailable_item(),
        Err(_) => return unavailable(),
    };
    if i64::from(input.quantity) > variant.available_quantity {
        return insufficient_stock();
    }
    if sqlx::query(
        r#"
        UPDATE cart_lines
        SET quantity = $3, unit_price_minor = $4, currency = $5, updated_at = now()
        WHERE id = $1 AND cart_id = $2
        "#,
    )
    .bind(line_id)
    .bind(session.id)
    .bind(input.quantity)
    .bind(variant.price_minor)
    .bind(variant.currency)
    .execute(&mut *transaction)
    .await
    .is_err()
        || touch_cart(&mut transaction, session.id).await.is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    cart_response(&pool, session).await
}

#[utoipa::path(
    delete,
    path = "/api/cart/items/{line_id}",
    tag = "cart",
    params(("line_id" = Uuid, Path), ("Idempotency-Key" = String, Header)),
    responses(
        (status = 200, body = Cart),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn remove_item(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(line_id): Path<Uuid>,
) -> Response {
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return invalid_idempotency_key();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let session = match resolve_cart(&pool, jar, state.secure_cookies).await {
        Ok(session) => session,
        Err(_) => return unavailable(),
    };
    let signature = format!("remove:{line_id}");
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    match claim_mutation(&mut transaction, session.id, &idempotency_hash, &signature).await {
        Ok(MutationClaim::Replay) => {
            if transaction.commit().await.is_err() {
                return unavailable();
            }
            return cart_response(&pool, session).await;
        }
        Ok(MutationClaim::Conflict) => return idempotency_conflict(),
        Ok(MutationClaim::New) => {}
        Err(_) => return unavailable(),
    }
    let deleted = match sqlx::query("DELETE FROM cart_lines WHERE id = $1 AND cart_id = $2")
        .bind(line_id)
        .bind(session.id)
        .execute(&mut *transaction)
        .await
    {
        Ok(result) => result.rows_affected(),
        Err(_) => return unavailable(),
    };
    if deleted == 0 {
        return line_not_found();
    }
    if reset_empty_currency(&mut transaction, session.id)
        .await
        .is_err()
        || touch_cart(&mut transaction, session.id).await.is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    cart_response(&pool, session).await
}

#[utoipa::path(
    post,
    path = "/api/cart/delivery",
    tag = "cart",
    params(("Idempotency-Key" = String, Header)),
    request_body = GuestCustomerRequest,
    responses(
        (status = 200, body = Cart),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn set_delivery(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<GuestCustomerRequest>,
) -> Response {
    if !valid_guest(&input) {
        return invalid_delivery();
    }
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return invalid_idempotency_key();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let account_customer_id = match optional_customer_id(&pool, &jar).await {
        Ok(customer_id) => customer_id,
        Err(_) => return unavailable(),
    };
    let session = match resolve_cart(&pool, jar, state.secure_cookies).await {
        Ok(session) => session,
        Err(_) => return unavailable(),
    };
    let signature = delivery_signature(&input);
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    match claim_mutation(&mut transaction, session.id, &idempotency_hash, &signature).await {
        Ok(MutationClaim::Replay) => {
            if transaction.commit().await.is_err() {
                return unavailable();
            }
            return cart_response(&pool, session).await;
        }
        Ok(MutationClaim::Conflict) => return idempotency_conflict(),
        Ok(MutationClaim::New) => {}
        Err(_) => return unavailable(),
    }
    let ownership = match cart_ownership(&mut transaction, session.id).await {
        Ok(ownership) => ownership,
        Err(_) => return unavailable(),
    };
    let reusable_guest =
        if account_customer_id.is_none() && ownership.customer_type.as_deref() == Some("guest") {
            ownership.customer_id
        } else {
            None
        };
    let customer_id = account_customer_id
        .or(reusable_guest)
        .unwrap_or_else(Uuid::now_v7);
    let registered = account_customer_id.is_some();
    if registered {
        let active: bool = match sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM customer_accounts account
                JOIN customers customer ON customer.id = account.customer_id
                WHERE customer.id = $1 AND customer.anonymized_at IS NULL
                  AND customer.retention_expires_at > now() AND account.disabled_at IS NULL
            )
            "#,
        )
        .bind(customer_id)
        .fetch_one(&mut *transaction)
        .await
        {
            Ok(active) => active,
            Err(_) => return unavailable(),
        };
        if !active {
            return unavailable();
        }
    } else if reusable_guest.is_some() {
        if update_guest(&mut transaction, customer_id, &input)
            .await
            .is_err()
        {
            return unavailable();
        }
    } else if insert_guest(&mut transaction, customer_id, &input)
        .await
        .is_err()
    {
        return unavailable();
    }
    let reusable_address = if ownership.customer_id == Some(customer_id) {
        ownership.shipping_address_id
    } else {
        None
    };
    let address_id = reusable_address.unwrap_or_else(Uuid::now_v7);
    let address_result = if reusable_address.is_some() {
        update_address(&mut transaction, address_id, customer_id, &input.address).await
    } else {
        insert_address(&mut transaction, address_id, customer_id, &input.address).await
    };
    if address_result.is_err()
        || sqlx::query(
            r#"
            UPDATE carts
            SET customer_id = $2, shipping_address_id = $3,
                updated_at = now(), expires_at = now() + interval '30 days'
            WHERE id = $1
            "#,
        )
        .bind(session.id)
        .bind(customer_id)
        .bind(address_id)
        .execute(&mut *transaction)
        .await
        .is_err()
        || audit_delivery(&mut transaction, session.id, customer_id, registered)
            .await
            .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    cart_response(&pool, session).await
}

#[utoipa::path(
    post,
    path = "/api/cart/discount",
    tag = "cart",
    params(("Idempotency-Key" = String, Header)),
    request_body = ApplyDiscountRequest,
    responses(
        (status = 200, body = Cart),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn apply_discount(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<ApplyDiscountRequest>,
) -> Response {
    let Some(code) = normalize_code(&input.code) else {
        return invalid_discount();
    };
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return invalid_idempotency_key();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let session = match resolve_cart(&pool, jar, state.secure_cookies).await {
        Ok(session) => session,
        Err(_) => return unavailable(),
    };
    let signature = format!("discount:apply:{code}");
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    match claim_mutation(&mut tx, session.id, &idempotency_hash, &signature).await {
        Ok(MutationClaim::Replay) => {
            if tx.commit().await.is_err() {
                return unavailable();
            }
            return cart_response(&pool, session).await;
        }
        Ok(MutationClaim::Conflict) => return idempotency_conflict(),
        Ok(MutationClaim::New) => {}
        Err(_) => return unavailable(),
    }
    let Some(discount_id) = (match find_by_code_in_transaction(&mut tx, &code).await {
        Ok(value) => value,
        Err(_) => return unavailable(),
    }) else {
        return invalid_discount();
    };
    let cart: Option<(Option<String>, Option<Uuid>, i64)> = sqlx::query_as(
        r#"SELECT cart.currency::text, cart.customer_id,
             COALESCE(sum(line.unit_price_minor * line.quantity),0)::bigint
           FROM carts cart LEFT JOIN cart_lines line ON line.cart_id = cart.id
           WHERE cart.id = $1 AND cart.status = 'active'
           GROUP BY cart.id"#,
    )
    .bind(session.id)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten();
    let Some((Some(currency), customer_id, subtotal_minor)) = cart else {
        return invalid_discount();
    };
    if evaluate_in_transaction(&mut tx, discount_id, subtotal_minor, &currency, customer_id)
        .await
        .is_err()
    {
        return invalid_discount();
    }
    if sqlx::query("UPDATE carts SET discount_id = $2, updated_at = now(), expires_at = now() + interval '30 days' WHERE id = $1")
        .bind(session.id).bind(discount_id).execute(&mut *tx).await.is_err()
        || tx.commit().await.is_err()
    { return unavailable() }
    cart_response(&pool, session).await
}

#[utoipa::path(
    delete,
    path = "/api/cart/discount",
    tag = "cart",
    params(("Idempotency-Key" = String, Header)),
    responses(
        (status = 200, body = Cart),
        (status = 409, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn remove_discount(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Response {
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return invalid_idempotency_key();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let session = match resolve_cart(&pool, jar, state.secure_cookies).await {
        Ok(session) => session,
        Err(_) => return unavailable(),
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    match claim_mutation(&mut tx, session.id, &idempotency_hash, "discount:remove").await {
        Ok(MutationClaim::Replay) => {
            if tx.commit().await.is_err() {
                return unavailable();
            }
            return cart_response(&pool, session).await;
        }
        Ok(MutationClaim::Conflict) => return idempotency_conflict(),
        Ok(MutationClaim::New) => {}
        Err(_) => return unavailable(),
    }
    if sqlx::query("UPDATE carts SET discount_id = NULL, updated_at = now(), expires_at = now() + interval '30 days' WHERE id = $1")
        .bind(session.id).execute(&mut *tx).await.is_err() || tx.commit().await.is_err()
    { return unavailable() }
    cart_response(&pool, session).await
}

async fn resolve_cart(
    pool: &PgPool,
    jar: CookieJar,
    secure: bool,
) -> Result<CartSession, sqlx::Error> {
    if let Some(cookie) = jar.get(CART_COOKIE) {
        let token_hash: [u8; 32] = Sha256::digest(cookie.value().as_bytes()).into();
        if let Some(id) = sqlx::query_scalar(
            "SELECT id FROM carts WHERE token_hash = $1 AND status = 'active' AND expires_at > now()",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(pool)
        .await?
        {
            return Ok(CartSession { id, jar });
        }
    }
    let id = Uuid::now_v7();
    let token = new_token();
    let token_hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    sqlx::query("INSERT INTO carts (id, token_hash) VALUES ($1, $2)")
        .bind(id)
        .bind(token_hash.as_slice())
        .execute(pool)
        .await?;
    Ok(CartSession {
        id,
        jar: jar.add(cart_cookie(token, secure)),
    })
}

async fn cart_response(pool: &PgPool, session: CartSession) -> Response {
    cart_response_with_status(pool, session, StatusCode::OK).await
}

async fn cart_response_with_status(
    pool: &PgPool,
    session: CartSession,
    status: StatusCode,
) -> Response {
    match load_cart(pool, session.id).await {
        Ok(cart) => no_store((status, session.jar, Json(cart)).into_response()),
        Err(_) => unavailable(),
    }
}

async fn load_cart(pool: &PgPool, cart_id: Uuid) -> Result<Cart, sqlx::Error> {
    let repriced: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT line.id
        FROM cart_lines line
        JOIN product_variants variant ON variant.id = line.variant_id
        WHERE line.cart_id = $1
          AND (line.unit_price_minor <> variant.price_minor OR line.currency <> variant.currency)
        "#,
    )
    .bind(cart_id)
    .fetch_all(pool)
    .await?;
    sqlx::query(
        r#"
        UPDATE cart_lines line
        SET unit_price_minor = variant.price_minor,
            currency = variant.currency,
            updated_at = CASE
                WHEN line.unit_price_minor <> variant.price_minor OR line.currency <> variant.currency
                THEN now() ELSE line.updated_at END
        FROM product_variants variant
        WHERE line.cart_id = $1 AND variant.id = line.variant_id
        "#,
    )
    .bind(cart_id)
    .execute(pool)
    .await?;
    let cart = sqlx::query_as::<_, CartRow>(
        r#"
        SELECT currency::text AS currency, customer_id, discount_id,
               to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS expires_at
        FROM carts WHERE id = $1 AND status = 'active'
        "#,
    )
    .bind(cart_id)
    .fetch_one(pool)
    .await?;
    let rows = sqlx::query_as::<_, CartLineRow>(
        r#"
        SELECT line.id, line.variant_id, product.slug AS product_slug,
               product.title AS product_title, product.status AS product_status,
               variant.title AS variant_title, variant.sku, line.quantity,
               line.unit_price_minor, line.currency::text AS currency,
               inventory.available_quantity,
               (
                   SELECT '/api/media/' || media.id || '/thumbnail'
                   FROM product_media relation
                   JOIN media_assets media ON media.id = relation.media_asset_id
                   WHERE relation.product_id = product.id AND media.status = 'ready'
                   ORDER BY relation.position, media.id LIMIT 1
               ) AS image_url
        FROM cart_lines line
        JOIN product_variants variant ON variant.id = line.variant_id
        JOIN products product ON product.id = variant.product_id
        JOIN inventory_items inventory ON inventory.variant_id = variant.id
        WHERE line.cart_id = $1
        ORDER BY line.created_at, line.id
        "#,
    )
    .bind(cart_id)
    .fetch_all(pool)
    .await?;
    let delivery = delivery_for(pool, cart_id).await?;
    let mut issues = Vec::new();
    let mut items = Vec::with_capacity(rows.len());
    let mut item_count = 0_i64;
    let mut subtotal_minor = 0_i64;
    for row in rows {
        let published = row.product_status == "active";
        let enough_stock = row.available_quantity >= i64::from(row.quantity);
        if !published {
            issues.push(CartIssue {
                code: "product_unavailable".into(),
                line_id: Some(row.id),
                message: format!("{} is no longer available.", row.product_title),
            });
        } else if !enough_stock {
            issues.push(CartIssue {
                code: "quantity_unavailable".into(),
                line_id: Some(row.id),
                message: format!(
                    "Only {} of {} is currently available.",
                    row.available_quantity, row.product_title
                ),
            });
        }
        if repriced.contains(&row.id) {
            issues.push(CartIssue {
                code: "price_changed".into(),
                line_id: Some(row.id),
                message: format!("The price of {} was updated.", row.product_title),
            });
        }
        if cart.currency.as_deref() != Some(row.currency.as_str()) {
            issues.push(CartIssue {
                code: "currency_changed".into(),
                line_id: Some(row.id),
                message: format!(
                    "{} is no longer sold in this cart's currency.",
                    row.product_title
                ),
            });
        }
        let line_total_minor = row.unit_price_minor.saturating_mul(i64::from(row.quantity));
        item_count = item_count.saturating_add(i64::from(row.quantity));
        subtotal_minor = subtotal_minor.saturating_add(line_total_minor);
        items.push(CartItem {
            id: row.id,
            variant_id: row.variant_id,
            product_slug: row.product_slug,
            product_title: row.product_title,
            variant_title: row.variant_title,
            sku: row.sku,
            quantity: row.quantity,
            unit_price_minor: row.unit_price_minor,
            currency: row.currency,
            line_total_minor,
            available_quantity: row.available_quantity,
            available: published && enough_stock,
            image_url: row.image_url,
        });
    }
    let mut discount = None;
    let mut discount_minor = 0;
    if let Some(discount_id) = cart.discount_id {
        match evaluate(
            pool,
            discount_id,
            subtotal_minor,
            cart.currency.as_deref().unwrap_or_default(),
            cart.customer_id,
        )
        .await
        {
            Ok(value) => {
                discount_minor = value.amount_minor;
                discount = Some(AppliedDiscount {
                    code: value.code,
                    kind: value.kind,
                    amount_minor: value.amount_minor,
                });
            }
            Err(EvaluationError::Unavailable) => issues.push(CartIssue {
                code: "discount_unavailable".into(),
                line_id: None,
                message:
                    "The discount no longer applies to this cart. Remove it or choose another code."
                        .into(),
            }),
            Err(EvaluationError::Database(error)) => return Err(error),
        }
    }
    let total_minor = subtotal_minor.saturating_sub(discount_minor);
    let checkout_ready = !items.is_empty() && issues.is_empty() && delivery.is_some();
    Ok(Cart {
        id: cart_id,
        currency: cart.currency,
        items,
        item_count,
        subtotal_minor,
        discount,
        discount_minor,
        total_minor,
        checkout_ready,
        issues,
        delivery,
        expires_at: cart.expires_at,
    })
}

async fn active_variant(
    transaction: &mut Transaction<'_, Postgres>,
    variant_id: Uuid,
) -> Result<Option<VariantForCart>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT variant.price_minor, variant.currency::text AS currency,
               inventory.available_quantity
        FROM product_variants variant
        JOIN products product ON product.id = variant.product_id
        JOIN inventory_items inventory ON inventory.variant_id = variant.id
        WHERE variant.id = $1 AND product.status = 'active'
        FOR UPDATE OF variant, inventory
        "#,
    )
    .bind(variant_id)
    .fetch_optional(&mut **transaction)
    .await
}

async fn ensure_currency(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
    currency: &str,
) -> Result<(), Response> {
    let current: Option<String> =
        sqlx::query_scalar("SELECT currency::text FROM carts WHERE id = $1 FOR UPDATE")
            .bind(cart_id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(|_| unavailable())?;
    if current
        .as_deref()
        .is_some_and(|current| current != currency)
    {
        return Err(currency_mismatch());
    }
    if current.is_none() {
        sqlx::query("UPDATE carts SET currency = $2 WHERE id = $1")
            .bind(cart_id)
            .bind(currency)
            .execute(&mut **transaction)
            .await
            .map_err(|_| unavailable())?;
    }
    Ok(())
}

async fn reset_empty_currency(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE carts SET currency = NULL, discount_id = NULL WHERE id = $1 AND NOT EXISTS (SELECT 1 FROM cart_lines WHERE cart_id = $1)",
    )
    .bind(cart_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn touch_cart(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE carts SET updated_at = now(), expires_at = now() + interval '30 days' WHERE id = $1",
    )
    .bind(cart_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn claim_mutation(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
    idempotency_hash: &[u8; 32],
    signature: &str,
) -> Result<MutationClaim, sqlx::Error> {
    let request_hash: [u8; 32] = Sha256::digest(signature.as_bytes()).into();
    let inserted = sqlx::query(
        r#"
        INSERT INTO cart_mutations (cart_id, idempotency_hash, request_hash)
        VALUES ($1, $2, $3) ON CONFLICT DO NOTHING
        "#,
    )
    .bind(cart_id)
    .bind(idempotency_hash.as_slice())
    .bind(request_hash.as_slice())
    .execute(&mut **transaction)
    .await?
    .rows_affected();
    if inserted == 1 {
        return Ok(MutationClaim::New);
    }
    let existing: Vec<u8> = sqlx::query_scalar(
        "SELECT request_hash FROM cart_mutations WHERE cart_id = $1 AND idempotency_hash = $2",
    )
    .bind(cart_id)
    .bind(idempotency_hash.as_slice())
    .fetch_one(&mut **transaction)
    .await?;
    Ok(if existing.as_slice() == request_hash {
        MutationClaim::Replay
    } else {
        MutationClaim::Conflict
    })
}

async fn optional_customer_id(pool: &PgPool, jar: &CookieJar) -> Result<Option<Uuid>, sqlx::Error> {
    let Some(cookie) = jar.get(CUSTOMER_SESSION_COOKIE) else {
        return Ok(None);
    };
    let token_hash = hash_token(cookie.value());
    sqlx::query_scalar(
        r#"
        SELECT customer.id
        FROM customer_sessions session
        JOIN customer_accounts account ON account.customer_id = session.customer_id
        JOIN customers customer ON customer.id = account.customer_id
        WHERE session.token_hash = $1 AND session.revoked_at IS NULL
          AND session.expires_at > now() AND account.disabled_at IS NULL
          AND customer.anonymized_at IS NULL AND customer.retention_expires_at > now()
        "#,
    )
    .bind(token_hash.as_slice())
    .fetch_optional(pool)
    .await
}

async fn cart_ownership(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
) -> Result<CartOwnership, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT cart.customer_id, cart.shipping_address_id, customer.customer_type
        FROM carts cart LEFT JOIN customers customer ON customer.id = cart.customer_id
        WHERE cart.id = $1 FOR UPDATE OF cart
        "#,
    )
    .bind(cart_id)
    .fetch_one(&mut **transaction)
    .await
}

async fn insert_guest(
    transaction: &mut Transaction<'_, Postgres>,
    customer_id: Uuid,
    input: &GuestCustomerRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO customers (id, email, first_name, last_name, phone) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(customer_id)
    .bind(input.email.trim().to_ascii_lowercase())
    .bind(input.first_name.trim())
    .bind(input.last_name.trim())
    .bind(input.phone.trim())
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO audit_log (action, entity_type, entity_id) VALUES ('customer.guest_create', 'customer', $1)",
    )
    .bind(customer_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn update_guest(
    transaction: &mut Transaction<'_, Postgres>,
    customer_id: Uuid,
    input: &GuestCustomerRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE customers SET email = $2, first_name = $3, last_name = $4, phone = $5,
            retention_expires_at = now() + interval '24 months', updated_at = now()
        WHERE id = $1 AND customer_type = 'guest' AND anonymized_at IS NULL
        "#,
    )
    .bind(customer_id)
    .bind(input.email.trim().to_ascii_lowercase())
    .bind(input.first_name.trim())
    .bind(input.last_name.trim())
    .bind(input.phone.trim())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_address(
    transaction: &mut Transaction<'_, Postgres>,
    address_id: Uuid,
    customer_id: Uuid,
    input: &CustomerAddressInput,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO customer_addresses (
            id, customer_id, recipient_name, line1, line2, city, region,
            postal_code, country_code, phone
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(address_id)
    .bind(customer_id)
    .bind(input.recipient_name.trim())
    .bind(input.line1.trim())
    .bind(input.line2.trim())
    .bind(input.city.trim())
    .bind(input.region.trim())
    .bind(input.postal_code.trim())
    .bind(input.country_code.trim().to_ascii_uppercase())
    .bind(input.phone.trim())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn update_address(
    transaction: &mut Transaction<'_, Postgres>,
    address_id: Uuid,
    customer_id: Uuid,
    input: &CustomerAddressInput,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE customer_addresses
        SET recipient_name = $3, line1 = $4, line2 = $5, city = $6, region = $7,
            postal_code = $8, country_code = $9, phone = $10, updated_at = now()
        WHERE id = $1 AND customer_id = $2
        "#,
    )
    .bind(address_id)
    .bind(customer_id)
    .bind(input.recipient_name.trim())
    .bind(input.line1.trim())
    .bind(input.line2.trim())
    .bind(input.city.trim())
    .bind(input.region.trim())
    .bind(input.postal_code.trim())
    .bind(input.country_code.trim().to_ascii_uppercase())
    .bind(input.phone.trim())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn audit_delivery(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
    customer_id: Uuid,
    registered: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_customer_id, action, entity_type, entity_id)
        VALUES ($1, 'cart.delivery_set', 'cart', $2)
        "#,
    )
    .bind(registered.then_some(customer_id))
    .bind(cart_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn delivery_for(pool: &PgPool, cart_id: Uuid) -> Result<Option<CartDelivery>, sqlx::Error> {
    let row = sqlx::query_as::<_, DeliveryRow>(
        r#"
        SELECT customer.id AS customer_id, address.id AS address_id,
               customer.email::text AS email, customer.first_name, customer.last_name,
               customer.phone AS customer_phone, address.recipient_name, address.line1,
               address.line2, address.city, address.region, address.postal_code,
               address.country_code::text AS country_code, address.phone AS address_phone
        FROM carts cart
        JOIN customers customer ON customer.id = cart.customer_id
        JOIN customer_addresses address ON address.id = cart.shipping_address_id
          AND address.customer_id = customer.id
        WHERE cart.id = $1 AND customer.anonymized_at IS NULL
          AND customer.retention_expires_at > now()
        "#,
    )
    .bind(cart_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| CartDelivery {
        customer_id: row.customer_id,
        address_id: row.address_id,
        email: row.email,
        first_name: row.first_name,
        last_name: row.last_name,
        phone: row.customer_phone,
        address: CartAddress {
            recipient_name: row.recipient_name,
            line1: row.line1,
            line2: row.line2,
            city: row.city,
            region: row.region,
            postal_code: row.postal_code,
            country_code: row.country_code,
            phone: row.address_phone,
        },
    }))
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

fn delivery_signature(input: &GuestCustomerRequest) -> String {
    format!(
        "delivery:{}",
        serde_json::to_string(input).expect("cart delivery requests always serialize")
    )
}

fn new_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn cart_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((CART_COOKIE, token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/api")
        .max_age(Duration::days(CART_DAYS))
        .build()
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

fn invalid_quantity() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_cart_quantity",
        "Cart quantities must be between 1 and 99.",
    )
}

fn invalid_idempotency_key() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_idempotency_key",
        "Provide a 16–128 character Idempotency-Key for this cart change.",
    )
}

fn idempotency_conflict() -> Response {
    error(
        StatusCode::CONFLICT,
        "idempotency_conflict",
        "That Idempotency-Key was already used for a different cart change.",
    )
}

fn insufficient_stock() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "insufficient_stock",
        "The requested quantity is not currently available.",
    )
}

fn unavailable_item() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "cart_item_unavailable",
        "That product option is not currently available.",
    )
}

fn line_not_found() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "cart_line_not_found",
        "That item is not in this cart.",
    )
}

fn currency_mismatch() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "cart_currency_mismatch",
        "A cart can contain products in only one currency.",
    )
}

fn invalid_delivery() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_cart_delivery",
        "Provide a valid contact and delivery address.",
    )
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "cart_unavailable",
        "The cart is temporarily unavailable.",
    )
}

fn invalid_discount() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorBody::new(
            "discount_unavailable",
            "That discount cannot be applied to this cart.",
        )),
    )
        .into_response()
}
