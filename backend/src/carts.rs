use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    packlink::{PackageItem, PacklinkPackage, packages_for_items},
    settings::{
        PricingError, ShippingSelection, TaxSelection, evaluate as evaluate_commercial,
        evaluate_packlink,
    },
};

const CART_COOKIE: &str = "knitprint_cart";
const CART_DAYS: i64 = 30;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct AddCartItemRequest {
    pub variant_id: Uuid,
    pub quantity: i32,
    #[serde(default)]
    pub customization: Option<Value>,
    #[serde(default)]
    pub customization_media_asset_id: Option<Uuid>,
    #[serde(default)]
    pub customization_media_asset_ids: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateCartItemRequest {
    pub quantity: i32,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ApplyDiscountRequest {
    pub code: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct SelectShippingMethodRequest {
    pub shipping_method_id: Uuid,
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
    pub shipping_methods: Vec<ShippingSelection>,
    pub shipping: Option<ShippingSelection>,
    pub shipping_minor: i64,
    pub tax: Option<TaxSelection>,
    pub tax_minor: i64,
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
    pub customization: Option<Value>,
    pub customization_media_asset_id: Option<Uuid>,
    pub customization_media_asset_ids: Vec<Uuid>,
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
    shipping_method_id: Option<Uuid>,
    shipping_quote_id: Option<Uuid>,
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
    image_url: Option<String>,
    customization: Option<Value>,
    customization_media_asset_id: Option<Uuid>,
    customization_media_asset_ids: Vec<Uuid>,
}

#[derive(FromRow)]
struct CartPackageItemRow {
    quantity: i64,
    shipping_weight_grams: i32,
    shipping_width_cm: i32,
    shipping_length_cm: i32,
    shipping_height_cm: i32,
    shipping_empty_weight_grams: i32,
    shipping_units_per_package: i32,
    shipping_profile_configured: bool,
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
    personalization_mode: String,
    print_areas: Value,
    personalization_views: Value,
    text_max_characters: i32,
    text_min_size: i32,
    text_max_size: i32,
    allowed_fonts: Value,
    allowed_colors: Value,
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
        r##"
        WITH claimed AS (
            SELECT id FROM carts
            WHERE status = 'expired' OR (status = 'active' AND expires_at <= now())
            ORDER BY expires_at, id
            FOR UPDATE SKIP LOCKED
            LIMIT $1
        )
        DELETE FROM carts cart USING claimed WHERE cart.id = claimed.id
        "##,
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
    if !(1..=100).contains(&input.quantity) || !valid_customization(input.customization.as_ref()) {
        return invalid_quantity();
    }
    let mut media_ids = input.customization_media_asset_ids.clone();
    if let Some(media_id) = input.customization_media_asset_id
        && !media_ids.contains(&media_id)
    {
        media_ids.push(media_id);
    }
    media_ids.sort_unstable();
    media_ids.dedup();
    if media_ids.len() > 48 {
        return invalid_customization();
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
    let signature = format!(
        "add:{}:{}:{:?}:{:?}",
        input.variant_id, input.quantity, input.customization, media_ids
    );
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
    if !customization_allowed(&variant, input.customization.as_ref(), &media_ids) {
        return invalid_customization();
    }
    for media_id in &media_ids {
        let ready: bool = match sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media_assets WHERE id=$1 AND status='ready' AND NOT EXISTS (SELECT 1 FROM product_media WHERE media_asset_id=$1))")
            .bind(media_id).fetch_one(&mut *transaction).await { Ok(value) => value, Err(_) => return unavailable() };
        if !ready {
            return invalid_customization();
        }
    }
    let existing_quantity: i32 = match sqlx::query_scalar(
        "SELECT quantity FROM cart_lines WHERE cart_id = $1 AND variant_id = $2 AND customization IS NULL FOR UPDATE",
    )
    .bind(session.id)
    .bind(input.variant_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(quantity) => if input.customization.is_none() { quantity.unwrap_or(0) } else { 0 },
        Err(_) => return unavailable(),
    };
    let Some(resulting_quantity) = existing_quantity.checked_add(input.quantity) else {
        return invalid_quantity();
    };
    if resulting_quantity > 100 {
        return invalid_quantity();
    }
    if let Err(error) = ensure_currency(&mut transaction, session.id, &variant.currency).await {
        return match error {
            EnsureCurrencyError::Unavailable => unavailable(),
            EnsureCurrencyError::Mismatch => currency_mismatch(),
        };
    }
    if sqlx::query(
        r##"
        INSERT INTO cart_lines (id, cart_id, variant_id, quantity, unit_price_minor, currency, customization, customization_media_asset_id, customization_media_asset_ids)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (cart_id, variant_id) WHERE customization IS NULL DO UPDATE
        SET quantity = EXCLUDED.quantity,
            unit_price_minor = EXCLUDED.unit_price_minor,
            currency = EXCLUDED.currency,
            updated_at = now()
        "##,
    )
    .bind(Uuid::now_v7())
    .bind(session.id)
    .bind(input.variant_id)
    .bind(resulting_quantity)
    .bind(variant.price_minor)
    .bind(&variant.currency)
    .bind(&input.customization)
    .bind(media_ids.first().copied())
    .bind(&media_ids)
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
    if !(1..=100).contains(&input.quantity) {
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
        || sqlx::query("DELETE FROM cart_shipping_quotes WHERE cart_id=$1")
            .bind(session.id)
            .execute(&mut *transaction)
            .await
            .is_err()
        || sqlx::query(
            r#"
            UPDATE carts
            SET customer_id = $2, shipping_address_id = $3,
                shipping_method_id = NULL, shipping_quote_id = NULL,
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
    path = "/api/cart/shipping-quotes",
    tag = "cart",
    responses(
        (status = 200, body = Cart),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn refresh_shipping_quotes(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    let session = match resolve_cart(pool, jar, state.secure_cookies).await {
        Ok(session) => session,
        Err(_) => return unavailable(),
    };
    if !state.packlink.enabled() {
        return cart_response(pool, session).await;
    }
    let delivery = match delivery_for(pool, session.id).await {
        Ok(Some(delivery)) => delivery,
        Ok(None) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "delivery_required",
                "Add a delivery address before requesting shipping prices.",
            );
        }
        Err(_) => return unavailable(),
    };
    let currency: Option<String> = match sqlx::query_scalar(
        "SELECT currency::text FROM carts WHERE id=$1 AND status='active'",
    )
    .bind(session.id)
    .fetch_one(pool)
    .await
    {
        Ok(currency) => currency,
        Err(_) => return unavailable(),
    };
    let Some(currency) = currency else {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "empty_cart",
            "Add a product before requesting shipping prices.",
        );
    };
    let packages = match cart_packages(pool, session.id).await {
        Ok(Some(packages)) => packages,
        Ok(None) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "shipping_package_invalid",
                "Review the product shipping dimensions before requesting prices.",
            );
        }
        Err(_) => return unavailable(),
    };
    let request_hash = state.packlink.request_hash(
        &delivery.address.country_code,
        &delivery.address.postal_code,
        &packages,
    );
    let cached: bool = match sqlx::query_scalar(
        r#"SELECT EXISTS(
        SELECT 1 FROM cart_shipping_quotes
        WHERE cart_id=$1 AND request_hash=$2 AND expires_at>now() AND currency=$3
        )"#,
    )
    .bind(session.id)
    .bind(request_hash.as_slice())
    .bind(&currency)
    .fetch_one(pool)
    .await
    {
        Ok(cached) => cached,
        Err(_) => return unavailable(),
    };
    if cached {
        return cart_response(pool, session).await;
    }
    let quotes = match state
        .packlink
        .quotes(
            &delivery.address.country_code,
            &delivery.address.postal_code,
            &packages,
        )
        .await
    {
        Ok(quotes) => quotes,
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "packlink_unavailable",
                "Shipping prices are temporarily unavailable. Try again in a moment.",
            );
        }
    };
    let quotes = quotes
        .into_iter()
        .filter(|quote| quote.currency == currency)
        .collect::<Vec<_>>();
    if quotes.is_empty() {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "packlink_no_services",
            "No delivery service is available for this address and package.",
        );
    }
    let previous_selection: Option<(String, bool, bool)> = match sqlx::query_as(
        r#"SELECT quote.service_id,quote.departure_dropoff,quote.destination_dropoff
        FROM carts cart
        JOIN cart_shipping_quotes quote ON quote.id=cart.shipping_quote_id
        WHERE cart.id=$1"#,
    )
    .bind(session.id)
    .fetch_optional(pool)
    .await
    {
        Ok(selection) => selection,
        Err(_) => return unavailable(),
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if sqlx::query("DELETE FROM cart_shipping_quotes WHERE cart_id=$1")
        .bind(session.id)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return unavailable();
    }
    let mut cheapest_id = None;
    let mut matching_id = None;
    for quote in quotes {
        let quote_id = Uuid::now_v7();
        cheapest_id.get_or_insert(quote_id);
        if previous_selection.as_ref().is_some_and(|selection| {
            selection.0 == quote.service_id
                && selection.1 == quote.departure_dropoff
                && selection.2 == quote.destination_dropoff
        }) {
            matching_id = Some(quote_id);
        }
        if sqlx::query(
            r#"INSERT INTO cart_shipping_quotes (
            id,cart_id,service_id,carrier_name,service_name,amount_minor,currency,
            departure_dropoff,destination_dropoff,transit_hours,request_hash,expires_at
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,now()+interval '15 minutes')"#,
        )
        .bind(quote_id)
        .bind(session.id)
        .bind(&quote.service_id)
        .bind(&quote.carrier_name)
        .bind(&quote.service_name)
        .bind(quote.amount_minor)
        .bind(&quote.currency)
        .bind(quote.departure_dropoff)
        .bind(quote.destination_dropoff)
        .bind(quote.transit_hours)
        .bind(request_hash.as_slice())
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            return unavailable();
        }
    }
    if sqlx::query(
        "UPDATE carts SET shipping_method_id=NULL,shipping_quote_id=$2,updated_at=now() WHERE id=$1",
    )
    .bind(session.id)
    .bind(matching_id.or(cheapest_id))
    .execute(&mut *transaction)
    .await
    .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    cart_response(pool, session).await
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

#[utoipa::path(
    post,
    path = "/api/cart/shipping-method",
    tag = "cart",
    params(("Idempotency-Key" = String, Header)),
    request_body = SelectShippingMethodRequest,
    responses(
        (status = 200, body = Cart),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn select_shipping_method(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<SelectShippingMethodRequest>,
) -> Response {
    let Some(idempotency_hash) = idempotency_hash(&headers) else {
        return invalid_idempotency_key();
    };
    let packlink_enabled = state.packlink.enabled();
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
    let signature = format!("shipping-method:{}", input.shipping_method_id);
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
    let eligibility_query = if packlink_enabled {
        r#"SELECT EXISTS (
        SELECT 1 FROM carts cart
        JOIN cart_shipping_quotes quote ON quote.id=$2 AND quote.cart_id=cart.id
        WHERE cart.id=$1 AND cart.status='active' AND quote.currency=cart.currency
          AND quote.expires_at>now()
        )"#
    } else {
        r#"SELECT EXISTS (
        SELECT 1 FROM carts cart
        JOIN customer_addresses address ON address.id=cart.shipping_address_id
        JOIN shipping_methods method ON method.id=$2 AND method.active
        JOIN shipping_zones zone ON zone.id=method.shipping_zone_id AND zone.active
        WHERE cart.id=$1 AND cart.status='active' AND method.currency=cart.currency
          AND (cardinality(zone.country_codes)=0 OR address.country_code::text=ANY(zone.country_codes))
        )"#
    };
    let eligible: bool = match sqlx::query_scalar(eligibility_query)
        .bind(session.id)
        .bind(input.shipping_method_id)
        .fetch_one(&mut *tx)
        .await
    {
        Ok(eligible) => eligible,
        Err(_) => return unavailable(),
    };
    if !eligible {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "shipping_method_unavailable",
            "Choose an available shipping method for this delivery address.",
        );
    }
    let update_query = if packlink_enabled {
        "UPDATE carts SET shipping_method_id=NULL,shipping_quote_id=$2,updated_at=now(),expires_at=now()+interval '30 days' WHERE id=$1"
    } else {
        "UPDATE carts SET shipping_method_id=$2,shipping_quote_id=NULL,updated_at=now(),expires_at=now()+interval '30 days' WHERE id=$1"
    };
    if sqlx::query(update_query)
        .bind(session.id)
        .bind(input.shipping_method_id)
        .execute(&mut *tx)
        .await
        .is_err()
        || tx.commit().await.is_err()
    {
        return unavailable();
    }
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
        SELECT currency::text AS currency, customer_id, discount_id, shipping_method_id, shipping_quote_id,
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
               (
                   SELECT '/api/media/' || media.id || '/thumbnail'
                   FROM product_media relation
                   JOIN media_assets media ON media.id = relation.media_asset_id
                   WHERE relation.product_id = product.id AND media.status = 'ready'
                   ORDER BY relation.position, media.id LIMIT 1
               ) AS image_url, line.customization, line.customization_media_asset_id,
               line.customization_media_asset_ids
        FROM cart_lines line
        JOIN product_variants variant ON variant.id = line.variant_id
        JOIN products product ON product.id = variant.product_id
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
        if !published {
            issues.push(CartIssue {
                code: "product_unavailable".into(),
                line_id: Some(row.id),
                message: format!("{} is no longer available.", row.product_title),
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
            available_quantity: 100,
            available: published,
            image_url: row.image_url,
            customization: row.customization,
            customization_media_asset_id: row.customization_media_asset_id,
            customization_media_asset_ids: row.customization_media_asset_ids,
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
    let merchandise_minor = subtotal_minor.saturating_sub(discount_minor);
    let mut shipping_methods = Vec::new();
    let mut shipping = None;
    let mut shipping_minor = 0;
    let mut tax = None;
    let mut tax_minor = 0;
    if let (Some(delivery), Some(currency)) = (&delivery, cart.currency.as_deref()) {
        let pricing_result = if crate::packlink::configured_in_environment() {
            let has_current_quotes: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM cart_shipping_quotes WHERE cart_id=$1 AND expires_at>now() AND currency=$2)",
            )
            .bind(cart_id)
            .bind(currency)
            .fetch_one(pool)
            .await?;
            if has_current_quotes {
                Some(
                    evaluate_packlink(
                        pool,
                        cart_id,
                        currency,
                        &delivery.address.country_code,
                        cart.shipping_quote_id,
                        merchandise_minor,
                    )
                    .await,
                )
            } else {
                None
            }
        } else {
            Some(
                evaluate_commercial(
                    pool,
                    currency,
                    &delivery.address.country_code,
                    cart.shipping_method_id,
                    merchandise_minor,
                )
                .await,
            )
        };
        if let Some(pricing_result) = pricing_result {
            match pricing_result {
                Ok(pricing) => {
                    shipping_methods = pricing.shipping_methods;
                    shipping_minor = pricing.shipping.amount_minor;
                    tax_minor = pricing.tax.amount_minor;
                    shipping = Some(pricing.shipping);
                    tax = Some(pricing.tax);
                }
                Err(PricingError::Unavailable) => issues.push(CartIssue {
                    code: "commercial_pricing_unavailable".into(),
                    line_id: None,
                    message: "Shipping or tax is not configured for this delivery address.".into(),
                }),
                Err(PricingError::Database(error)) => return Err(error),
            }
        }
    }
    let total_minor = merchandise_minor
        .saturating_add(shipping_minor)
        .saturating_add(tax_minor);
    let checkout_ready =
        !items.is_empty() && issues.is_empty() && delivery.is_some() && shipping.is_some();
    Ok(Cart {
        id: cart_id,
        currency: cart.currency,
        items,
        item_count,
        subtotal_minor,
        discount,
        discount_minor,
        shipping_methods,
        shipping,
        shipping_minor,
        tax,
        tax_minor,
        total_minor,
        checkout_ready,
        issues,
        delivery,
        expires_at: cart.expires_at,
    })
}

fn valid_customization(value: Option<&Value>) -> bool {
    value.is_none_or(|value| value.is_object() && value.to_string().len() <= 20_000)
}

fn valid_element_frame(value: &Value) -> bool {
    const FRAME_ROUNDING_TOLERANCE: f64 = 0.0001;
    let Some(x) = value.get("x").and_then(Value::as_f64) else {
        return false;
    };
    let Some(y) = value.get("y").and_then(Value::as_f64) else {
        return false;
    };
    let Some(width) = value.get("width").and_then(Value::as_f64) else {
        return false;
    };
    let Some(height) = value.get("height").and_then(Value::as_f64) else {
        return false;
    };
    x >= 0.0
        && y >= 0.0
        && width > 0.0
        && height > 0.0
        && x + width <= 100.0 + FRAME_ROUNDING_TOLERANCE
        && y + height <= 100.0 + FRAME_ROUNDING_TOLERANCE
}

fn valid_print_area_assignment(element: &Value, print_areas: &Value) -> bool {
    let Some(area_id) = element.get("area_id").and_then(Value::as_str) else {
        return false;
    };
    print_areas.as_array().is_some_and(|areas| {
        areas
            .iter()
            .any(|area| area.get("id").and_then(Value::as_str) == Some(area_id))
    })
}

fn customization_allowed(
    variant: &VariantForCart,
    customization: Option<&Value>,
    media_ids: &[Uuid],
) -> bool {
    let version = customization
        .and_then(|value| value.get("version"))
        .and_then(Value::as_i64)
        .unwrap_or(1);
    if version >= 5 {
        return view_customization_allowed(variant, customization, media_ids);
    }
    if version >= 4 {
        return area_customization_allowed(variant, customization, media_ids);
    }
    if media_ids.len() > 1 {
        return false;
    }
    let media_id = media_ids.first().copied();
    let photo = customization.and_then(|value| value.get("photo"));
    let text = customization.and_then(|value| value.get("text"));
    let has_personalization = photo.is_some() || text.is_some();
    let photo_upload_matches = photo.is_some() == media_id.is_some();
    let shape_valid = match variant.personalization_mode.as_str() {
        "none" => customization.is_none() && media_id.is_none(),
        "photo" => {
            text.is_none()
                && photo_upload_matches
                && (customization.is_none() || has_personalization)
        }
        "text" => {
            photo.is_none()
                && media_id.is_none()
                && (customization.is_none() || has_personalization)
        }
        "photo_text" => photo_upload_matches && (customization.is_none() || has_personalization),
        _ => false,
    };
    if !shape_valid {
        return false;
    }
    if let Some(photo) = photo {
        if !photo.is_object() {
            return false;
        }
        if version >= 2 {
            let crop_valid = photo
                .get("crop_x")
                .and_then(Value::as_f64)
                .is_some_and(|value| (0.0..=100.0).contains(&value))
                && photo
                    .get("crop_y")
                    .and_then(Value::as_f64)
                    .is_some_and(|value| (0.0..=100.0).contains(&value))
                && photo
                    .get("scale")
                    .and_then(Value::as_f64)
                    .is_some_and(|value| (1.0..=3.0).contains(&value));
            if !valid_element_frame(photo) || !crop_valid {
                return false;
            }
        }
        if version >= 3 && !valid_print_area_assignment(photo, &variant.print_areas) {
            return false;
        }
    }
    let Some(text) = text else {
        return true;
    };
    let Some(content) = text.get("content").and_then(Value::as_str) else {
        return false;
    };
    let Some(font) = text.get("font").and_then(Value::as_str) else {
        return false;
    };
    let Some(color) = text.get("color").and_then(Value::as_str) else {
        return false;
    };
    let Some(size) = text.get("size").and_then(Value::as_i64) else {
        return false;
    };
    let Some(x) = text.get("x").and_then(Value::as_f64) else {
        return false;
    };
    let Some(y) = text.get("y").and_then(Value::as_f64) else {
        return false;
    };
    !content.trim().is_empty()
        && content.chars().count() <= variant.text_max_characters as usize
        && (variant.text_min_size as i64..=variant.text_max_size as i64).contains(&size)
        && (0.0..=100.0).contains(&x)
        && (0.0..=100.0).contains(&y)
        && (version < 2 || valid_element_frame(text))
        && (version < 3 || valid_print_area_assignment(text, &variant.print_areas))
        && variant
            .allowed_fonts
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(font)))
        && variant
            .allowed_colors
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(color)))
}

fn view_customization_allowed(
    variant: &VariantForCart,
    customization: Option<&Value>,
    media_ids: &[Uuid],
) -> bool {
    if variant.personalization_mode == "none" {
        return customization.is_none() && media_ids.is_empty();
    }
    let Some(areas) = customization
        .and_then(|value| value.get("areas"))
        .and_then(Value::as_array)
    else {
        return customization.is_none() && media_ids.is_empty();
    };
    if areas.is_empty() || areas.len() > 48 {
        return false;
    }
    let Some(configured_views) = variant.personalization_views.as_array() else {
        return false;
    };
    let version = customization
        .and_then(|value| value.get("version"))
        .and_then(Value::as_i64)
        .unwrap_or(5);
    let mut seen_assignments = Vec::with_capacity(areas.len());
    let mut referenced_media_ids = Vec::new();
    for area in areas {
        let Some(view_id) = area.get("view_id").and_then(Value::as_str) else {
            return false;
        };
        let Some(area_id) = area.get("area_id").and_then(Value::as_str) else {
            return false;
        };
        if seen_assignments.contains(&(view_id, area_id)) {
            return false;
        }
        let Some(configured_view) = configured_views
            .iter()
            .find(|configured| configured.get("id").and_then(Value::as_str) == Some(view_id))
        else {
            return false;
        };
        let Some(configured_area) = configured_view
            .get("print_areas")
            .and_then(Value::as_array)
            .and_then(|configured_areas| {
                configured_areas.iter().find(|configured| {
                    configured.get("id").and_then(Value::as_str) == Some(area_id)
                })
            })
        else {
            return false;
        };
        if version >= 6 && !valid_measurement_snapshot(area, configured_view, configured_area) {
            return false;
        }
        if version >= 7 && !valid_article_reference_snapshot(area, configured_view, configured_area)
        {
            return false;
        }
        seen_assignments.push((view_id, area_id));
        let photo = area.get("photo");
        let text = area.get("text");
        let shape_valid = match variant.personalization_mode.as_str() {
            "photo" => photo.is_some() && text.is_none(),
            "text" => text.is_some() && photo.is_none(),
            "photo_text" => photo.is_some() || text.is_some(),
            _ => false,
        };
        if !shape_valid {
            return false;
        }
        if let Some(photo) = photo {
            let Some(media_id) = valid_area_photo(photo) else {
                return false;
            };
            referenced_media_ids.push(media_id);
        }
        if text.is_some_and(|text| !valid_area_text(text, variant)) {
            return false;
        }
    }
    referenced_media_ids.sort_unstable();
    referenced_media_ids.dedup();
    let mut supplied_media_ids = media_ids.to_vec();
    supplied_media_ids.sort_unstable();
    supplied_media_ids.dedup();
    referenced_media_ids == supplied_media_ids
}

fn valid_measurement_snapshot(
    customization_area: &Value,
    configured_view: &Value,
    configured_area: &Value,
) -> bool {
    let configured_dimension =
        |key: &str| configured_area.get(key).map_or(Some(20.0), Value::as_f64);
    let matches_dimension = |snapshot_key: &str, configured_key: &str| {
        customization_area
            .get(snapshot_key)
            .and_then(Value::as_f64)
            .zip(configured_dimension(configured_key))
            .is_some_and(|(snapshot, configured)| (snapshot - configured).abs() <= 0.01)
    };
    customization_area.get("view_label").and_then(Value::as_str)
        == configured_view.get("label").and_then(Value::as_str)
        && customization_area.get("area_label").and_then(Value::as_str)
            == configured_area.get("label").and_then(Value::as_str)
        && matches_dimension("print_width_cm", "physical_width_cm")
        && matches_dimension("print_height_cm", "physical_height_cm")
}

fn valid_article_reference_snapshot(
    customization_area: &Value,
    configured_view: &Value,
    configured_area: &Value,
) -> bool {
    let configured_reference = configured_view
        .get("article_reference")
        .filter(|reference| {
            reference
                .get("configured")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        });
    let Some(configured_reference) = configured_reference else {
        return customization_area.get("article_reference").is_none();
    };
    let Some(snapshot) = customization_area.get("article_reference") else {
        return false;
    };
    let configured = |key: &str| configured_reference.get(key).and_then(Value::as_f64);
    let area = |key: &str| configured_area.get(key).and_then(Value::as_f64);
    let Some((reference_x, reference_y, reference_width, reference_height)) = configured("x")
        .zip(configured("y"))
        .zip(configured("width").zip(configured("height")))
        .map(|((x, y), (width, height))| (x, y, width, height))
    else {
        return false;
    };
    let Some((area_x, area_y)) = area("x").zip(area("y")) else {
        return false;
    };
    let Some((physical_width, physical_height)) =
        configured("physical_width_cm").zip(configured("physical_height_cm"))
    else {
        return false;
    };
    let expected = [
        ("article_width_cm", physical_width),
        ("article_height_cm", physical_height),
        (
            "print_left_cm",
            physical_width * (area_x - reference_x) / reference_width,
        ),
        (
            "print_top_cm",
            physical_height * (area_y - reference_y) / reference_height,
        ),
    ];
    snapshot.is_object()
        && expected.iter().all(|(key, expected)| {
            snapshot
                .get(*key)
                .and_then(Value::as_f64)
                .is_some_and(|actual| (actual - expected).abs() <= 0.05)
        })
}

fn valid_area_text(text: &Value, variant: &VariantForCart) -> bool {
    let Some(content) = text.get("content").and_then(Value::as_str) else {
        return false;
    };
    let Some(font) = text.get("font").and_then(Value::as_str) else {
        return false;
    };
    let Some(color) = text.get("color").and_then(Value::as_str) else {
        return false;
    };
    let Some(size) = text.get("size").and_then(Value::as_i64) else {
        return false;
    };
    !content.trim().is_empty()
        && content.chars().count() <= variant.text_max_characters as usize
        && (variant.text_min_size as i64..=variant.text_max_size as i64).contains(&size)
        && valid_element_frame(text)
        && variant
            .allowed_fonts
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(font)))
        && variant
            .allowed_colors
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(color)))
}

fn valid_area_photo(photo: &Value) -> Option<Uuid> {
    let crop_valid = photo
        .get("crop_x")
        .and_then(Value::as_f64)
        .is_some_and(|value| (0.0..=100.0).contains(&value))
        && photo
            .get("crop_y")
            .and_then(Value::as_f64)
            .is_some_and(|value| (0.0..=100.0).contains(&value))
        && photo
            .get("scale")
            .and_then(Value::as_f64)
            .is_some_and(|value| (1.0..=3.0).contains(&value));
    if !photo.is_object() || !valid_element_frame(photo) || !crop_valid {
        return None;
    }
    photo
        .get("media_id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn area_customization_allowed(
    variant: &VariantForCart,
    customization: Option<&Value>,
    media_ids: &[Uuid],
) -> bool {
    if variant.personalization_mode == "none" {
        return customization.is_none() && media_ids.is_empty();
    }
    let Some(areas) = customization
        .and_then(|value| value.get("areas"))
        .and_then(Value::as_array)
    else {
        return customization.is_none() && media_ids.is_empty();
    };
    if areas.is_empty() || areas.len() > 8 {
        return false;
    }
    let Some(configured_areas) = variant.print_areas.as_array() else {
        return false;
    };
    let mut seen_area_ids: Vec<&str> = Vec::with_capacity(areas.len());
    let mut referenced_media_ids = Vec::new();
    for area in areas {
        let Some(area_id) = area.get("area_id").and_then(Value::as_str) else {
            return false;
        };
        if seen_area_ids.contains(&area_id)
            || !configured_areas
                .iter()
                .any(|configured| configured.get("id").and_then(Value::as_str) == Some(area_id))
        {
            return false;
        }
        seen_area_ids.push(area_id);
        let photo = area.get("photo");
        let text = area.get("text");
        let shape_valid = match variant.personalization_mode.as_str() {
            "photo" => photo.is_some() && text.is_none(),
            "text" => text.is_some() && photo.is_none(),
            "photo_text" => photo.is_some() || text.is_some(),
            _ => false,
        };
        if !shape_valid {
            return false;
        }
        if let Some(photo) = photo {
            let Some(media_id) = valid_area_photo(photo) else {
                return false;
            };
            referenced_media_ids.push(media_id);
        }
        if text.is_some_and(|text| !valid_area_text(text, variant)) {
            return false;
        }
    }
    referenced_media_ids.sort_unstable();
    referenced_media_ids.dedup();
    let mut supplied_media_ids = media_ids.to_vec();
    supplied_media_ids.sort_unstable();
    supplied_media_ids.dedup();
    referenced_media_ids == supplied_media_ids
}

async fn active_variant(
    transaction: &mut Transaction<'_, Postgres>,
    variant_id: Uuid,
) -> Result<Option<VariantForCart>, sqlx::Error> {
    sqlx::query_as(
        r##"
        SELECT variant.price_minor, variant.currency::text AS currency,
               COALESCE(personalization.mode, 'none') AS personalization_mode,
               COALESCE(personalization.print_areas, '[{"id":"area-1"}]'::jsonb) AS print_areas,
               COALESCE(personalization.views, '[{"id":"view-front","print_areas":[{"id":"area-1"}]}]'::jsonb) AS personalization_views,
               COALESCE(personalization.text_max_characters, 35) AS text_max_characters,
               COALESCE(personalization.text_min_size, 12) AS text_min_size,
               COALESCE(personalization.text_max_size, 72) AS text_max_size,
               COALESCE(personalization.allowed_fonts, '["Arial"]'::jsonb) AS allowed_fonts,
               COALESCE(personalization.allowed_colors, '["#111111"]'::jsonb) AS allowed_colors
        FROM product_variants variant
        JOIN products product ON product.id = variant.product_id
        LEFT JOIN product_personalization personalization ON personalization.product_id = product.id
        WHERE variant.id = $1 AND product.status = 'active'
        FOR UPDATE OF variant
        "##,
    )
    .bind(variant_id)
    .fetch_optional(&mut **transaction)
    .await
}

async fn ensure_currency(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
    currency: &str,
) -> Result<(), EnsureCurrencyError> {
    let current: Option<String> =
        sqlx::query_scalar("SELECT currency::text FROM carts WHERE id = $1 FOR UPDATE")
            .bind(cart_id)
            .fetch_one(&mut **transaction)
            .await
            .map_err(|_| EnsureCurrencyError::Unavailable)?;
    if current
        .as_deref()
        .is_some_and(|current| current != currency)
    {
        return Err(EnsureCurrencyError::Mismatch);
    }
    if current.is_none() {
        sqlx::query("UPDATE carts SET currency = $2 WHERE id = $1")
            .bind(cart_id)
            .bind(currency)
            .execute(&mut **transaction)
            .await
            .map_err(|_| EnsureCurrencyError::Unavailable)?;
    }
    Ok(())
}

enum EnsureCurrencyError {
    Unavailable,
    Mismatch,
}

async fn reset_empty_currency(
    transaction: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE carts SET currency = NULL, discount_id = NULL, shipping_method_id = NULL WHERE id = $1 AND NOT EXISTS (SELECT 1 FROM cart_lines WHERE cart_id = $1)",
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
    sqlx::query("DELETE FROM cart_shipping_quotes WHERE cart_id = $1")
        .bind(cart_id)
        .execute(&mut **transaction)
        .await?;
    sqlx::query(
        "UPDATE carts SET shipping_quote_id = NULL, updated_at = now(), expires_at = now() + interval '30 days' WHERE id = $1",
    )
    .bind(cart_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn cart_packages(
    pool: &PgPool,
    cart_id: Uuid,
) -> Result<Option<Vec<PacklinkPackage>>, sqlx::Error> {
    let rows = sqlx::query_as::<_, CartPackageItemRow>(
        r#"
        SELECT sum(line.quantity)::bigint AS quantity,
               product.shipping_weight_grams,
               COALESCE(profile.width_cm, product.shipping_width_cm) AS shipping_width_cm,
               COALESCE(profile.length_cm, product.shipping_length_cm) AS shipping_length_cm,
               COALESCE(profile.height_cm, product.shipping_height_cm) AS shipping_height_cm,
               COALESCE(profile.empty_weight_grams, 0) AS shipping_empty_weight_grams,
               product.shipping_units_per_package,
               (product.shipping_profile_configured AND COALESCE(profile.active, false)) AS shipping_profile_configured
        FROM cart_lines line
        JOIN product_variants variant ON variant.id = line.variant_id
        JOIN products product ON product.id = variant.product_id
        LEFT JOIN shipping_package_profiles profile ON profile.id=product.shipping_package_profile_id
        WHERE line.cart_id = $1
        GROUP BY product.id, product.shipping_weight_grams, profile.width_cm,
                 profile.length_cm, profile.height_cm, profile.empty_weight_grams,
                 profile.active, product.shipping_width_cm, product.shipping_length_cm,
                 product.shipping_height_cm, product.shipping_units_per_package,
                 product.shipping_profile_configured
        ORDER BY product.id
        "#,
    )
    .bind(cart_id)
    .fetch_all(pool)
    .await?;
    if rows.iter().any(|row| !row.shipping_profile_configured) {
        return Ok(None);
    }
    let items = rows
        .into_iter()
        .map(|row| PackageItem {
            quantity: row.quantity,
            unit_weight_grams: row.shipping_weight_grams,
            width_cm: row.shipping_width_cm,
            length_cm: row.shipping_length_cm,
            height_cm: row.shipping_height_cm,
            empty_weight_grams: row.shipping_empty_weight_grams,
            units_per_package: row.shipping_units_per_package,
        })
        .collect::<Vec<_>>();
    Ok(packages_for_items(&items))
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
        "Cart quantities must be between 1 and 100.",
    )
}

fn invalid_customization() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_customization",
        "The personalization does not match the options allowed for this product.",
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

#[cfg(test)]
mod personalization_tests {
    use super::*;

    fn variant(mode: &str) -> VariantForCart {
        VariantForCart {
            price_minor: 1000,
            currency: "EUR".into(),
            personalization_mode: mode.into(),
            print_areas: serde_json::json!([
                { "id": "area-1" },
                { "id": "pocket-side" }
            ]),
            personalization_views: serde_json::json!([
                { "id": "view-front", "label": "Frente", "article_reference": { "configured": true, "x": 1000, "y": 1000, "width": 8000, "height": 8000, "physical_width_cm": 40, "physical_height_cm": 50 }, "print_areas": [{ "id": "area-1", "label": "Peito", "x": 2000, "y": 2000, "width": 6000, "height": 5600, "physical_width_cm": 30, "physical_height_cm": 35 }] },
                { "id": "view-back", "label": "Costas", "print_areas": [{ "id": "pocket-side", "label": "Costas", "physical_width_cm": 28, "physical_height_cm": 32 }] }
            ]),
            text_max_characters: 35,
            text_min_size: 12,
            text_max_size: 72,
            allowed_fonts: serde_json::json!(["Roboto"]),
            allowed_colors: serde_json::json!(["#111111"]),
        }
    }

    fn text_at(x: f64, y: f64) -> Value {
        serde_json::json!({ "text": { "content": "Olá", "font": "Roboto", "color": "#111111", "size": 24, "x": x, "y": y } })
    }

    fn framed_text(x: f64, y: f64, width: f64, height: f64) -> Value {
        serde_json::json!({
            "version": 2,
            "text": { "content": "Olá", "font": "Roboto", "color": "#111111", "size": 24, "x": x, "y": y, "width": width, "height": height }
        })
    }

    #[test]
    fn personalization_is_optional_for_personalizable_products() {
        for mode in ["photo", "text", "photo_text"] {
            assert!(customization_allowed(&variant(mode), None, &[]));
        }
    }

    #[test]
    fn combined_mode_accepts_partial_or_complete_personalization() {
        let media_id = Uuid::new_v4();
        let photo = serde_json::json!({ "version": 2, "photo": { "x": 10, "y": 10, "width": 80, "height": 60, "crop_x": 50, "crop_y": 50, "scale": 1 } });
        let both = serde_json::json!({
            "version": 2,
            "photo": { "x": 10, "y": 10, "width": 80, "height": 60, "crop_x": 50, "crop_y": 50, "scale": 1 },
            "text": { "content": "Olá", "font": "Roboto", "color": "#111111", "size": 24, "x": 15, "y": 72, "width": 70, "height": 20 }
        });
        assert!(customization_allowed(
            &variant("photo_text"),
            Some(&text_at(50.0, 50.0)),
            &[]
        ));
        assert!(customization_allowed(
            &variant("photo_text"),
            Some(&photo),
            &[media_id]
        ));
        assert!(customization_allowed(
            &variant("photo_text"),
            Some(&both),
            &[media_id]
        ));
    }

    #[test]
    fn text_position_must_remain_inside_the_print_area() {
        assert!(customization_allowed(
            &variant("text"),
            Some(&text_at(0.0, 100.0)),
            &[]
        ));
        assert!(!customization_allowed(
            &variant("text"),
            Some(&text_at(101.0, 50.0)),
            &[]
        ));
        assert!(customization_allowed(
            &variant("text"),
            Some(&framed_text(10.0, 20.0, 60.0, 30.0)),
            &[]
        ));
        assert!(!customization_allowed(
            &variant("text"),
            Some(&framed_text(50.0, 20.0, 60.0, 30.0)),
            &[]
        ));
    }

    #[test]
    fn photo_frame_and_crop_must_be_valid() {
        let media_id = Uuid::new_v4();
        let outside = serde_json::json!({
            "version": 2,
            "photo": { "x": 30, "y": 10, "width": 80, "height": 60, "crop_x": 50, "crop_y": 50, "scale": 1 }
        });
        let excessive_zoom = serde_json::json!({
            "version": 2,
            "photo": { "x": 10, "y": 10, "width": 80, "height": 60, "crop_x": 50, "crop_y": 50, "scale": 4 }
        });
        assert!(!customization_allowed(
            &variant("photo"),
            Some(&outside),
            &[media_id]
        ));
        assert!(!customization_allowed(
            &variant("photo"),
            Some(&excessive_zoom),
            &[media_id]
        ));
    }

    #[test]
    fn frame_validation_tolerates_browser_rounding_at_the_boundary() {
        assert!(valid_element_frame(&serde_json::json!({
            "x": 5.0000001,
            "y": 10.0000001,
            "width": 95.0,
            "height": 90.0
        })));
        assert!(!valid_element_frame(&serde_json::json!({
            "x": 5.0,
            "y": 10.0,
            "width": 95.01,
            "height": 90.0
        })));
    }

    #[test]
    fn version_three_elements_must_use_an_available_print_area() {
        let valid = serde_json::json!({
            "version": 3,
            "text": { "content": "Olá", "font": "Roboto", "color": "#111111", "size": 24, "area_id": "pocket-side", "x": 10, "y": 10, "width": 60, "height": 30 }
        });
        let unknown = serde_json::json!({
            "version": 3,
            "text": { "content": "Olá", "font": "Roboto", "color": "#111111", "size": 24, "area_id": "back", "x": 10, "y": 10, "width": 60, "height": 30 }
        });
        assert!(customization_allowed(&variant("text"), Some(&valid), &[]));
        assert!(!customization_allowed(
            &variant("text"),
            Some(&unknown),
            &[]
        ));
    }

    #[test]
    fn every_print_area_can_have_its_own_photo_and_text() {
        let first_media_id = Uuid::new_v4();
        let second_media_id = Uuid::new_v4();
        let customization = serde_json::json!({
            "version": 4,
            "areas": [
                {
                    "area_id": "area-1",
                    "photo": { "media_id": first_media_id, "x": 5, "y": 5, "width": 90, "height": 50, "crop_x": 50, "crop_y": 50, "scale": 1 },
                    "text": { "content": "Frente", "font": "Roboto", "color": "#111111", "size": 24, "x": 10, "y": 58, "width": 80, "height": 40 }
                },
                {
                    "area_id": "pocket-side",
                    "photo": { "media_id": second_media_id, "x": 5, "y": 5, "width": 90, "height": 50, "crop_x": 50, "crop_y": 50, "scale": 1 },
                    "text": { "content": "Bolso", "font": "Roboto", "color": "#111111", "size": 18, "x": 10, "y": 58, "width": 80, "height": 40 }
                }
            ]
        });
        assert!(customization_allowed(
            &variant("photo_text"),
            Some(&customization),
            &[first_media_id, second_media_id]
        ));
        assert!(!customization_allowed(
            &variant("photo_text"),
            Some(&customization),
            &[first_media_id]
        ));
    }

    #[test]
    fn every_product_view_has_independent_print_areas() {
        let media_id = Uuid::new_v4();
        let customization = serde_json::json!({
            "version": 5,
            "areas": [
                {
                    "view_id": "view-front",
                    "area_id": "area-1",
                    "photo": { "media_id": media_id, "x": 5, "y": 5, "width": 90, "height": 50, "crop_x": 50, "crop_y": 50, "scale": 1 }
                },
                {
                    "view_id": "view-back",
                    "area_id": "pocket-side",
                    "text": { "content": "Costas", "font": "Roboto", "color": "#111111", "size": 24, "x": 10, "y": 20, "width": 80, "height": 40 }
                }
            ]
        });
        assert!(customization_allowed(
            &variant("photo_text"),
            Some(&customization),
            &[media_id]
        ));
        let wrong_view = serde_json::json!({
            "version": 5,
            "areas": [{
                "view_id": "view-front",
                "area_id": "pocket-side",
                "text": { "content": "Costas", "font": "Roboto", "color": "#111111", "size": 24, "x": 10, "y": 20, "width": 80, "height": 40 }
            }]
        });
        assert!(!customization_allowed(
            &variant("photo_text"),
            Some(&wrong_view),
            &[]
        ));
    }

    #[test]
    fn physical_print_measurements_are_trusted_only_when_they_match_the_product() {
        let valid = serde_json::json!({
            "version": 6,
            "areas": [{
                "view_id": "view-front",
                "view_label": "Frente",
                "area_id": "area-1",
                "area_label": "Peito",
                "print_width_cm": 30,
                "print_height_cm": 35,
                "text": { "content": "Olá", "font": "Roboto", "color": "#111111", "size": 24, "x": 10, "y": 20, "width": 80, "height": 40 }
            }]
        });
        assert!(customization_allowed(
            &variant("photo_text"),
            Some(&valid),
            &[]
        ));
        let mut tampered = valid;
        tampered["areas"][0]["print_width_cm"] = serde_json::json!(60);
        assert!(!customization_allowed(
            &variant("photo_text"),
            Some(&tampered),
            &[]
        ));
    }

    #[test]
    fn physical_article_offsets_are_trusted_only_when_they_match_the_calibration() {
        let valid = serde_json::json!({
            "version": 7,
            "areas": [{
                "view_id": "view-front",
                "view_label": "Frente",
                "area_id": "area-1",
                "area_label": "Peito",
                "print_width_cm": 30,
                "print_height_cm": 35,
                "article_reference": {
                    "article_width_cm": 40,
                    "article_height_cm": 50,
                    "print_left_cm": 5,
                    "print_top_cm": 6.25
                },
                "text": { "content": "Olá", "font": "Roboto", "color": "#111111", "size": 24, "x": 10, "y": 20, "width": 80, "height": 40 }
            }]
        });
        assert!(customization_allowed(
            &variant("photo_text"),
            Some(&valid),
            &[]
        ));
        let mut tampered = valid;
        tampered["areas"][0]["article_reference"]["print_left_cm"] = serde_json::json!(15);
        assert!(!customization_allowed(
            &variant("photo_text"),
            Some(&tampered),
            &[]
        ));
    }
}
