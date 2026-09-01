use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, Postgres, Transaction};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
    orders::OrderSummary,
};

const CUSTOMERS_READ: &str = "customers.read";
const ORDERS_READ: &str = "orders.read";

#[derive(Deserialize, Serialize, ToSchema)]
pub struct GuestCustomerRequest {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(default)]
    pub phone: String,
    pub address: CustomerAddressInput,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct CustomerAddressInput {
    pub recipient_name: String,
    pub line1: String,
    #[serde(default)]
    pub line2: String,
    pub city: String,
    #[serde(default)]
    pub region: String,
    pub postal_code: String,
    pub country_code: String,
    #[serde(default)]
    pub phone: String,
}

#[derive(Serialize, ToSchema)]
pub struct GuestCustomerReceipt {
    pub customer_id: Uuid,
    pub address_id: Uuid,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct CustomerSummary {
    pub id: Uuid,
    pub customer_type: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub address_count: i64,
    pub created_at: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct CustomerAddress {
    pub id: Uuid,
    pub address_type: String,
    pub recipient_name: String,
    pub line1: String,
    pub line2: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country_code: String,
    pub phone: String,
}

#[derive(Serialize, ToSchema)]
pub struct CustomerDetail {
    pub id: Uuid,
    pub customer_type: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub retention_expires_at: String,
    pub created_at: String,
    pub addresses: Vec<CustomerAddress>,
    pub order_count: i64,
}

#[derive(FromRow)]
struct CustomerDetailRow {
    id: Uuid,
    customer_type: String,
    email: String,
    first_name: String,
    last_name: String,
    phone: String,
    retention_expires_at: String,
    created_at: String,
}

#[derive(Deserialize, IntoParams)]
pub struct CustomerQuery {
    pub q: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/customers/guest",
    tag = "customers",
    params((
        "Idempotency-Key" = String,
        Header,
        description = "16–128 visible characters reused for retries of one submission"
    )),
    request_body = GuestCustomerRequest,
    responses(
        (status = 200, body = GuestCustomerReceipt, description = "Existing idempotent submission"),
        (status = 201, body = GuestCustomerReceipt),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn create_guest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<GuestCustomerRequest>,
) -> Response {
    if !valid_guest(&input) {
        return invalid_input();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let Some(idempotency_key) = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| {
            (16..=128).contains(&value.len())
                && value.chars().all(|character| character.is_ascii_graphic())
        })
    else {
        return invalid_idempotency_key();
    };
    let idempotency_hash: [u8; 32] = Sha256::digest(idempotency_key.as_bytes()).into();
    let advisory_lock = i64::from_be_bytes(
        idempotency_hash[..8]
            .try_into()
            .expect("SHA-256 always has at least eight bytes"),
    );
    let customer_id = Uuid::now_v7();
    let address_id = Uuid::now_v7();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(advisory_lock)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return unavailable();
    }
    match sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT customer_id, address_id
        FROM guest_customer_requests
        WHERE idempotency_hash = $1
        "#,
    )
    .bind(idempotency_hash.as_slice())
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some((customer_id, address_id))) => {
            if transaction.commit().await.is_err() {
                return unavailable();
            }
            return Json(GuestCustomerReceipt {
                customer_id,
                address_id,
            })
            .into_response();
        }
        Ok(None) => {}
        Err(_) => return unavailable(),
    }
    if sqlx::query(
        r#"
        INSERT INTO customers (id, email, first_name, last_name, phone)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(customer_id)
    .bind(input.email.trim().to_ascii_lowercase())
    .bind(input.first_name.trim())
    .bind(input.last_name.trim())
    .bind(input.phone.trim())
    .execute(&mut *transaction)
    .await
    .is_err()
        || sqlx::query(
            r#"
            INSERT INTO customer_addresses (
                id, customer_id, recipient_name, line1, line2, city, region,
                postal_code, country_code, phone
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(address_id)
        .bind(customer_id)
        .bind(input.address.recipient_name.trim())
        .bind(input.address.line1.trim())
        .bind(input.address.line2.trim())
        .bind(input.address.city.trim())
        .bind(input.address.region.trim())
        .bind(input.address.postal_code.trim())
        .bind(input.address.country_code.trim().to_ascii_uppercase())
        .bind(input.address.phone.trim())
        .execute(&mut *transaction)
        .await
        .is_err()
        || sqlx::query(
            r#"
            INSERT INTO guest_customer_requests (idempotency_hash, customer_id, address_id)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(idempotency_hash.as_slice())
        .bind(customer_id)
        .bind(address_id)
        .execute(&mut *transaction)
        .await
        .is_err()
        || audit(
            &mut transaction,
            None,
            "customer.guest_create",
            Some(customer_id),
        )
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    (
        StatusCode::CREATED,
        Json(GuestCustomerReceipt {
            customer_id,
            address_id,
        }),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/api/admin/customers",
    params(CustomerQuery),
    tag = "admin customers",
    responses(
        (status = 200, body = [CustomerSummary]),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn list(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Query(query): Query<CustomerQuery>,
) -> Response {
    if let Err(response) = require_capability(&actor, CUSTOMERS_READ) {
        return response.into_response();
    }
    let search = query.q.unwrap_or_default().trim().to_owned();
    if search.len() > 200 {
        return invalid_query();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let customers = match sqlx::query_as::<_, CustomerSummary>(
        r#"
        SELECT
            customer.id,
            customer.customer_type,
            customer.email::text AS email,
            customer.first_name,
            customer.last_name,
            count(address.id) AS address_count,
            to_char(customer.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM customers customer
        LEFT JOIN customer_addresses address ON address.customer_id = customer.id
        WHERE customer.anonymized_at IS NULL
          AND customer.retention_expires_at > now()
          AND (
            $1 = ''
            OR customer.email::text ILIKE '%' || $1 || '%'
            OR customer.first_name ILIKE '%' || $1 || '%'
            OR customer.last_name ILIKE '%' || $1 || '%'
            OR customer.search_document @@ plainto_tsquery('simple', $1)
          )
        GROUP BY customer.id
        ORDER BY customer.created_at DESC, customer.id
        LIMIT 100
        "#,
    )
    .bind(search)
    .fetch_all(&mut *transaction)
    .await
    {
        Ok(customers) => customers,
        Err(_) => return unavailable(),
    };
    if audit(
        &mut transaction,
        Some(actor.id),
        "customer.private_list",
        None,
    )
    .await
    .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    Json(customers).into_response()
}

#[utoipa::path(
    get,
    path = "/api/admin/customers/{customer_id}",
    params(("customer_id" = Uuid, Path)),
    tag = "admin customers",
    responses(
        (status = 200, body = CustomerDetail),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody)
    )
)]
pub async fn detail(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(customer_id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_capability(&actor, CUSTOMERS_READ) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let customer = match sqlx::query_as::<_, CustomerDetailRow>(
        r#"
        SELECT
            id,
            customer_type,
            email::text AS email,
            first_name,
            last_name,
            phone,
            to_char(retention_expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS retention_expires_at,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM customers
        WHERE id = $1 AND anonymized_at IS NULL AND retention_expires_at > now()
        "#,
    )
    .bind(customer_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(customer)) => customer,
        Ok(None) => return not_found(),
        Err(_) => return unavailable(),
    };
    let addresses = match sqlx::query_as::<_, CustomerAddress>(
        r#"
        SELECT
            id,
            address_type,
            recipient_name,
            line1,
            line2,
            city,
            region,
            postal_code,
            country_code::text AS country_code,
            phone
        FROM customer_addresses
        WHERE customer_id = $1
        ORDER BY created_at, id
        "#,
    )
    .bind(customer_id)
    .fetch_all(&mut *transaction)
    .await
    {
        Ok(addresses) => addresses,
        Err(_) => return unavailable(),
    };
    let order_count: i64 =
        match sqlx::query_scalar("SELECT count(*) FROM orders WHERE customer_id = $1")
            .bind(customer_id)
            .fetch_one(&mut *transaction)
            .await
        {
            Ok(count) => count,
            Err(_) => return unavailable(),
        };
    if audit(
        &mut transaction,
        Some(actor.id),
        "customer.private_view",
        Some(customer_id),
    )
    .await
    .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    Json(CustomerDetail {
        id: customer.id,
        customer_type: customer.customer_type,
        email: customer.email,
        first_name: customer.first_name,
        last_name: customer.last_name,
        phone: customer.phone,
        retention_expires_at: customer.retention_expires_at,
        created_at: customer.created_at,
        addresses,
        order_count,
    })
    .into_response()
}

#[utoipa::path(
    get,
    path = "/api/admin/customers/{customer_id}/orders",
    params(("customer_id" = Uuid, Path)),
    tag = "admin customers",
    responses(
        (status = 200, body = [OrderSummary]),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn order_history(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(customer_id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_capability(&actor, CUSTOMERS_READ) {
        return response.into_response();
    }
    if let Err(response) = require_capability(&actor, ORDERS_READ) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let customer_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM customers WHERE id = $1 AND anonymized_at IS NULL AND retention_expires_at > now())",
    )
    .bind(customer_id)
    .fetch_one(&mut *transaction)
    .await;
    match customer_exists {
        Ok(true) => {}
        Ok(false) => return not_found(),
        Err(_) => return unavailable(),
    }
    let orders = match sqlx::query_as::<_, OrderSummary>(
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
        WHERE order_record.customer_id = $1
        GROUP BY order_record.id
        ORDER BY order_record.created_at DESC, order_record.id DESC
        LIMIT 200
        "#,
    )
    .bind(customer_id)
    .fetch_all(&mut *transaction)
    .await
    {
        Ok(orders) => orders,
        Err(_) => return unavailable(),
    };
    if audit(
        &mut transaction,
        Some(actor.id),
        "customer.order_history_view",
        Some(customer_id),
    )
    .await
    .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    Json(orders).into_response()
}

async fn audit(
    transaction: &mut Transaction<'_, Postgres>,
    actor: Option<Uuid>,
    action: &str,
    customer_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id)
        VALUES ($1, $2, 'customer', $3)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(customer_id.map(|id| id.to_string()))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub(crate) fn valid_guest(input: &GuestCustomerRequest) -> bool {
    valid_email(&input.email)
        && valid_required(&input.first_name, 100)
        && valid_required(&input.last_name, 100)
        && input.phone.trim().len() <= 40
        && valid_required(&input.address.recipient_name, 200)
        && valid_required(&input.address.line1, 200)
        && input.address.line2.trim().len() <= 200
        && valid_required(&input.address.city, 120)
        && input.address.region.trim().len() <= 120
        && valid_required(&input.address.postal_code, 32)
        && input.address.country_code.trim().len() == 2
        && input
            .address
            .country_code
            .trim()
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        && input.address.phone.trim().len() <= 40
}

pub(crate) fn valid_email(value: &str) -> bool {
    let value = value.trim();
    let mut parts = value.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    value.len() <= 320
        && value.len() >= 3
        && !value.contains(char::is_whitespace)
        && parts.next().is_none()
        && !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && domain.split('.').count() >= 2
        && domain.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        })
}

fn valid_required(value: &str, maximum: usize) -> bool {
    let value = value.trim();
    !value.is_empty() && value.len() <= maximum
}

fn invalid_input() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_customer_details",
        "Provide valid customer contact and delivery details.",
    )
}

fn invalid_query() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_customer_query",
        "Customer search must contain at most 200 characters.",
    )
}

fn invalid_idempotency_key() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "idempotency_key_required",
        "Provide an Idempotency-Key header containing 16 to 128 visible characters.",
    )
}

fn not_found() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "customer_not_found",
        "The customer was not found.",
    )
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "database_unavailable",
        "Customer records are temporarily unavailable.",
    )
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}

#[cfg(test)]
mod tests {
    use super::{CustomerAddressInput, GuestCustomerRequest, valid_guest};

    fn valid_input() -> GuestCustomerRequest {
        GuestCustomerRequest {
            email: "guest@example.com".into(),
            first_name: "Ada".into(),
            last_name: "Lovelace".into(),
            phone: "".into(),
            address: CustomerAddressInput {
                recipient_name: "Ada Lovelace".into(),
                line1: "12 Loom Lane".into(),
                line2: "".into(),
                city: "Lisbon".into(),
                region: "".into(),
                postal_code: "1000-001".into(),
                country_code: "PT".into(),
                phone: "".into(),
            },
        }
    }

    #[test]
    fn validates_guest_contact_and_address_details() {
        assert!(valid_guest(&valid_input()));
        let mut input = valid_input();
        input.email = "not-an-email".into();
        assert!(!valid_guest(&input));
        let mut input = valid_input();
        input.email = "a@b.example@invalid".into();
        assert!(!valid_guest(&input));
        let mut input = valid_input();
        input.address.country_code = "Portugal".into();
        assert!(!valid_guest(&input));
    }
}
