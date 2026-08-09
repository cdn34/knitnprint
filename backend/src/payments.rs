use std::{collections::HashMap, env, future::Future, pin::Pin, sync::Arc, time::Duration};

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use thiserror::Error;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    config::Environment,
    error::ErrorBody,
    inventory::{commit_in_transaction, release_in_transaction},
};

const CART_COOKIE: &str = "knitprint_cart";
const WEBHOOK_TOLERANCE_SECONDS: i64 = 300;
const CHECKOUT_LIFETIME_SECONDS: i64 = 35 * 60;
const ABANDONED_GRACE_SECONDS: i64 = 60 * 60;
const STRIPE_API_VERSION: &str = "2026-02-25.clover";

pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProviderCheckout, PaymentProviderError>> + Send + 'a>>;

#[derive(Clone, Debug)]
pub struct ProviderCheckoutRequest {
    pub attempt_id: Uuid,
    pub order_id: Uuid,
    pub order_number: String,
    pub amount_minor: i64,
    pub currency: String,
    pub customer_email: String,
}

#[derive(Clone, Debug)]
pub struct ProviderCheckout {
    pub provider_payment_id: String,
    pub checkout_url: String,
    pub expires_at: i64,
}

#[derive(Debug, Error)]
#[error("payment provider request failed")]
pub struct PaymentProviderError;

pub trait PaymentProvider: Send + Sync {
    fn create_checkout(&self, request: ProviderCheckoutRequest) -> ProviderFuture<'_>;
}

#[derive(Clone, Default)]
pub struct PaymentService {
    provider: Option<Arc<dyn PaymentProvider>>,
    webhook_secret: Option<Arc<str>>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum PaymentConfigError {
    #[error(
        "STRIPE_SECRET_KEY, STRIPE_WEBHOOK_SECRET, and STOREFRONT_BASE_URL must be configured together"
    )]
    Incomplete,
    #[error("STRIPE_SECRET_KEY must start with sk_test_ or sk_live_")]
    InvalidSecretKey,
    #[error("STRIPE_WEBHOOK_SECRET must start with whsec_")]
    InvalidWebhookSecret,
    #[error("STOREFRONT_BASE_URL must be an absolute HTTP(S) URL without a path")]
    InvalidStorefrontUrl,
    #[error("production Stripe configuration requires HTTPS and a live secret key")]
    UnsafeProductionConfiguration,
    #[error("Stripe configuration is required in production")]
    MissingProductionConfiguration,
}

impl PaymentService {
    pub fn from_env(environment: Environment) -> Result<Self, PaymentConfigError> {
        Self::from_values(
            environment,
            env::var("STRIPE_SECRET_KEY").ok().as_deref(),
            env::var("STRIPE_WEBHOOK_SECRET").ok().as_deref(),
            env::var("STOREFRONT_BASE_URL").ok().as_deref(),
        )
    }

    fn from_values(
        environment: Environment,
        secret_key: Option<&str>,
        webhook_secret: Option<&str>,
        storefront_base_url: Option<&str>,
    ) -> Result<Self, PaymentConfigError> {
        let any_present = secret_key.is_some() || webhook_secret.is_some();
        if !any_present {
            return if environment == Environment::Production {
                Err(PaymentConfigError::MissingProductionConfiguration)
            } else {
                Ok(Self::default())
            };
        }
        let (Some(secret_key), Some(webhook_secret), Some(storefront_base_url)) =
            (secret_key, webhook_secret, storefront_base_url)
        else {
            return Err(PaymentConfigError::Incomplete);
        };
        if !(secret_key.starts_with("sk_test_") || secret_key.starts_with("sk_live_")) {
            return Err(PaymentConfigError::InvalidSecretKey);
        }
        if !webhook_secret.starts_with("whsec_") || webhook_secret.len() <= 6 {
            return Err(PaymentConfigError::InvalidWebhookSecret);
        }
        let valid_http = storefront_base_url.starts_with("http://")
            || storefront_base_url.starts_with("https://");
        let authority = storefront_base_url
            .split_once("://")
            .map(|(_, value)| value)
            .unwrap_or_default();
        if !valid_http
            || authority.is_empty()
            || authority.contains(['/', '?', '#'])
            || storefront_base_url.ends_with('/')
        {
            return Err(PaymentConfigError::InvalidStorefrontUrl);
        }
        if environment == Environment::Production
            && (!secret_key.starts_with("sk_live_") || !storefront_base_url.starts_with("https://"))
        {
            return Err(PaymentConfigError::UnsafeProductionConfiguration);
        }
        Ok(Self {
            provider: Some(Arc::new(StripeProvider {
                client: reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .map_err(|_| PaymentConfigError::Incomplete)?,
                secret_key: Arc::from(secret_key),
                storefront_base_url: Arc::from(storefront_base_url),
            })),
            webhook_secret: Some(Arc::from(webhook_secret)),
        })
    }

    pub fn with_provider(
        provider: Arc<dyn PaymentProvider>,
        webhook_secret: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            provider: Some(provider),
            webhook_secret: Some(webhook_secret.into()),
        }
    }

    pub fn enabled(&self) -> bool {
        self.provider.is_some()
    }
}

struct StripeProvider {
    client: reqwest::Client,
    secret_key: Arc<str>,
    storefront_base_url: Arc<str>,
}

#[derive(Deserialize)]
struct StripeCheckoutResponse {
    id: String,
    url: Option<String>,
    expires_at: i64,
}

impl PaymentProvider for StripeProvider {
    fn create_checkout(&self, request: ProviderCheckoutRequest) -> ProviderFuture<'_> {
        Box::pin(async move {
            let success_url = format!(
                "{}/cart?payment=return&order_id={}",
                self.storefront_base_url, request.order_id
            );
            let cancel_url = format!(
                "{}/cart?payment=cancelled&order_id={}",
                self.storefront_base_url, request.order_id
            );
            let expires_at = OffsetDateTime::now_utc().unix_timestamp() + CHECKOUT_LIFETIME_SECONDS;
            let order_id = request.order_id.to_string();
            let attempt_id = request.attempt_id.to_string();
            let amount = request.amount_minor.to_string();
            let currency = request.currency.to_ascii_lowercase();
            let description = format!("KnitPrint order {}", request.order_number);
            let form = [
                ("mode", "payment".to_owned()),
                ("success_url", success_url),
                ("cancel_url", cancel_url),
                ("client_reference_id", order_id.clone()),
                ("customer_email", request.customer_email),
                ("expires_at", expires_at.to_string()),
                ("line_items[0][quantity]", "1".to_owned()),
                ("line_items[0][price_data][currency]", currency),
                ("line_items[0][price_data][unit_amount]", amount),
                ("line_items[0][price_data][product_data][name]", description),
                ("metadata[order_id]", order_id.clone()),
                ("metadata[payment_attempt_id]", attempt_id.clone()),
                ("payment_intent_data[metadata][order_id]", order_id),
                (
                    "payment_intent_data[metadata][payment_attempt_id]",
                    attempt_id,
                ),
            ];
            let response = self
                .client
                .post("https://api.stripe.com/v1/checkout/sessions")
                .basic_auth(self.secret_key.as_ref(), Some(""))
                .header("Stripe-Version", STRIPE_API_VERSION)
                .header(
                    "Idempotency-Key",
                    format!("knitprint-checkout-{}", request.attempt_id),
                )
                .form(&form)
                .send()
                .await
                .map_err(|_| PaymentProviderError)?;
            if !response.status().is_success() {
                return Err(PaymentProviderError);
            }
            let checkout: StripeCheckoutResponse =
                response.json().await.map_err(|_| PaymentProviderError)?;
            let checkout_url = checkout.url.ok_or(PaymentProviderError)?;
            if !checkout_url.starts_with("https://") {
                return Err(PaymentProviderError);
            }
            Ok(ProviderCheckout {
                provider_payment_id: checkout.id,
                checkout_url,
                expires_at: checkout.expires_at,
            })
        })
    }
}

#[derive(Serialize, ToSchema)]
pub struct PaymentOptions {
    pub stripe: bool,
    pub manual: bool,
}

#[derive(Serialize, ToSchema)]
pub struct PaymentCheckout {
    pub order_id: Uuid,
    pub provider: String,
    pub checkout_url: String,
    pub expires_at: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct PaymentAttempt {
    pub id: Uuid,
    pub attempt_number: i32,
    pub provider: String,
    pub status: String,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct PaymentStatusEvent {
    pub id: Uuid,
    pub event_type: String,
    pub provider_status: String,
    pub detail: String,
    pub created_at: String,
}

#[utoipa::path(
    get,
    path = "/api/payments/options",
    tag = "payments",
    responses((status = 200, body = PaymentOptions))
)]
pub async fn options(State(state): State<AppState>) -> Json<PaymentOptions> {
    Json(PaymentOptions {
        stripe: state.payments.enabled(),
        manual: state.manual_payments_enabled,
    })
}

#[utoipa::path(
    post,
    path = "/api/orders/{order_id}/payment",
    tag = "payments",
    params(("order_id" = Uuid, Path)),
    responses(
        (status = 200, body = PaymentCheckout),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn start_checkout(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(order_id): Path<Uuid>,
) -> Response {
    let Some(cookie) = jar.get(CART_COOKIE) else {
        return payment_error(
            StatusCode::NOT_FOUND,
            "order_not_found",
            "The order was not found.",
        );
    };
    let Some(provider) = state.payments.provider.clone() else {
        return payment_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "stripe_unavailable",
            "Card checkout is temporarily unavailable.",
        );
    };
    let Some(pool) = state.database else {
        return payments_unavailable();
    };
    let token_hash: [u8; 32] = Sha256::digest(cookie.value().as_bytes()).into();
    match prepare_checkout(&pool, order_id, token_hash, provider).await {
        Ok(checkout) => Json(checkout).into_response(),
        Err(StartError::NotFound) => payment_error(
            StatusCode::NOT_FOUND,
            "order_not_found",
            "The order was not found.",
        ),
        Err(StartError::Conflict) => payment_error(
            StatusCode::CONFLICT,
            "payment_state_conflict",
            "This order cannot start card checkout in its current state.",
        ),
        Err(StartError::Provider) => payment_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "stripe_unavailable",
            "Card checkout is temporarily unavailable. Retry with the same order.",
        ),
        Err(StartError::Database) => payments_unavailable(),
    }
}

#[derive(FromRow)]
struct CheckoutRecord {
    payment_id: Uuid,
    order_number: String,
    amount_minor: i64,
    currency: String,
    customer_email: String,
    order_status: String,
    payment_status: String,
    provider: String,
}

#[derive(FromRow)]
struct AttemptRecord {
    id: Uuid,
    provider_payment_id: Option<String>,
    checkout_url: Option<String>,
    expires_at: Option<String>,
}

enum StartError {
    NotFound,
    Conflict,
    Provider,
    Database,
}

async fn prepare_checkout(
    pool: &PgPool,
    order_id: Uuid,
    token_hash: [u8; 32],
    provider: Arc<dyn PaymentProvider>,
) -> Result<PaymentCheckout, StartError> {
    let mut transaction = pool.begin().await.map_err(|_| StartError::Database)?;
    let record = sqlx::query_as::<_, CheckoutRecord>(
        r#"
        SELECT payment.id AS payment_id, order_record.order_number,
               payment.amount_minor, payment.currency::text AS currency,
               order_record.customer_email, order_record.order_status,
               payment.status AS payment_status, payment.provider
        FROM orders order_record
        JOIN carts cart ON cart.id = order_record.cart_id
        JOIN order_payments payment ON payment.order_id = order_record.id
        WHERE order_record.id = $1 AND cart.token_hash = $2
        FOR UPDATE OF payment
        "#,
    )
    .bind(order_id)
    .bind(token_hash.as_slice())
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| StartError::Database)?
    .ok_or(StartError::NotFound)?;
    if record.provider != "stripe"
        || record.order_status != "pending"
        || record.payment_status != "pending"
    {
        return Err(StartError::Conflict);
    }
    let existing = sqlx::query_as::<_, AttemptRecord>(
        r#"
        SELECT id, provider_payment_id, checkout_url,
               to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS expires_at
        FROM payment_attempts WHERE order_payment_id = $1 AND attempt_number = 1
        "#,
    )
    .bind(record.payment_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|_| StartError::Database)?;
    if let Some(attempt) = &existing
        && let (Some(provider_payment_id), Some(checkout_url), Some(expires_at)) = (
            attempt.provider_payment_id.as_ref(),
            attempt.checkout_url.as_ref(),
            attempt.expires_at.as_ref(),
        )
    {
        let _ = provider_payment_id;
        transaction
            .commit()
            .await
            .map_err(|_| StartError::Database)?;
        return Ok(PaymentCheckout {
            order_id,
            provider: "stripe".to_owned(),
            checkout_url: checkout_url.clone(),
            expires_at: expires_at.clone(),
        });
    }
    let attempt_id = existing.map_or_else(Uuid::now_v7, |attempt| attempt.id);
    sqlx::query(
        r#"
        INSERT INTO payment_attempts (id, order_payment_id, attempt_number, provider)
        VALUES ($1, $2, 1, 'stripe')
        ON CONFLICT (order_payment_id, attempt_number) DO NOTHING
        "#,
    )
    .bind(attempt_id)
    .bind(record.payment_id)
    .execute(&mut *transaction)
    .await
    .map_err(|_| StartError::Database)?;
    transaction
        .commit()
        .await
        .map_err(|_| StartError::Database)?;

    let checkout = provider
        .create_checkout(ProviderCheckoutRequest {
            attempt_id,
            order_id,
            order_number: record.order_number,
            amount_minor: record.amount_minor,
            currency: record.currency,
            customer_email: record.customer_email,
        })
        .await
        .map_err(|_| StartError::Provider)?;
    persist_checkout(pool, record.payment_id, attempt_id, order_id, checkout).await
}

async fn persist_checkout(
    pool: &PgPool,
    payment_id: Uuid,
    attempt_id: Uuid,
    order_id: Uuid,
    checkout: ProviderCheckout,
) -> Result<PaymentCheckout, StartError> {
    let mut transaction = pool.begin().await.map_err(|_| StartError::Database)?;
    let updated = sqlx::query(
        r#"
        UPDATE payment_attempts
        SET provider_payment_id = $2, status = 'pending', checkout_url = $3,
            expires_at = to_timestamp($4), updated_at = now()
        WHERE id = $1 AND checkout_url IS NULL
        "#,
    )
    .bind(attempt_id)
    .bind(&checkout.provider_payment_id)
    .bind(&checkout.checkout_url)
    .bind(checkout.expires_at)
    .execute(&mut *transaction)
    .await
    .map_err(|_| StartError::Database)?;
    sqlx::query(
        "UPDATE order_payments SET provider_payment_id = $2, updated_at = now() WHERE id = $1",
    )
    .bind(payment_id)
    .bind(&checkout.provider_payment_id)
    .execute(&mut *transaction)
    .await
    .map_err(|_| StartError::Database)?;
    if updated.rows_affected() == 1 {
        sqlx::query(
            r#"
            INSERT INTO payment_status_events (
                id, order_payment_id, payment_attempt_id, provider, event_type, provider_status, detail
            ) VALUES ($1, $2, $3, 'stripe', 'payment.checkout_created', 'pending', 'Customer redirected to Stripe Checkout.')
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(payment_id)
        .bind(attempt_id)
        .execute(&mut *transaction)
        .await
        .map_err(|_| StartError::Database)?;
    }
    transaction
        .commit()
        .await
        .map_err(|_| StartError::Database)?;
    Ok(PaymentCheckout {
        order_id,
        provider: "stripe".to_owned(),
        checkout_url: checkout.checkout_url,
        expires_at: OffsetDateTime::from_unix_timestamp(checkout.expires_at)
            .map_err(|_| StartError::Provider)?
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|_| StartError::Provider)?,
    })
}

#[derive(Deserialize)]
struct StripeEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
    data: StripeEventData,
}

#[derive(Deserialize)]
struct StripeEventData {
    object: StripeCheckoutObject,
}

#[derive(Deserialize)]
struct StripeCheckoutObject {
    id: String,
    object: String,
    payment_status: Option<String>,
    metadata: HashMap<String, String>,
}

#[derive(Clone, Copy)]
enum WebhookTransition {
    Processing,
    Succeeded,
    Failed,
    Expired,
}

#[utoipa::path(
    post,
    path = "/api/payments/stripe/webhook",
    tag = "payments",
    request_body(content = String, content_type = "application/json"),
    responses(
        (status = 200, description = "Verified event accepted"),
        (status = 400, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(secret) = state.payments.webhook_secret.as_deref() else {
        return payments_unavailable();
    };
    let Some(signature) = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
    else {
        return invalid_webhook();
    };
    if !verify_signature(
        secret.as_bytes(),
        signature,
        &body,
        OffsetDateTime::now_utc().unix_timestamp(),
    ) {
        return invalid_webhook();
    }
    let Ok(event) = serde_json::from_slice::<StripeEvent>(&body) else {
        return invalid_webhook();
    };
    if event.data.object.object != "checkout.session" {
        return StatusCode::OK.into_response();
    }
    let transition = match event.event_type.as_str() {
        "checkout.session.completed" => {
            if matches!(
                event.data.object.payment_status.as_deref(),
                Some("paid" | "no_payment_required")
            ) {
                WebhookTransition::Succeeded
            } else {
                WebhookTransition::Processing
            }
        }
        "checkout.session.async_payment_succeeded" => WebhookTransition::Succeeded,
        "checkout.session.async_payment_failed" => WebhookTransition::Failed,
        "checkout.session.expired" => WebhookTransition::Expired,
        _ => return StatusCode::OK.into_response(),
    };
    let Some(order_id) = event
        .data
        .object
        .metadata
        .get("order_id")
        .and_then(|value| Uuid::parse_str(value).ok())
    else {
        return invalid_webhook();
    };
    let Some(pool) = state.database else {
        return payments_unavailable();
    };
    let payload_hash: [u8; 32] = Sha256::digest(&body).into();
    match apply_webhook(
        &pool,
        &event.id,
        &event.event_type,
        &event.data.object.id,
        order_id,
        transition,
        payload_hash,
    )
    .await
    {
        Ok(()) => StatusCode::OK.into_response(),
        Err(_) => payments_unavailable(),
    }
}

#[derive(FromRow)]
struct WebhookRecord {
    payment_id: Uuid,
    attempt_id: Uuid,
    order_status: String,
    payment_status: String,
    attempt_status: String,
    order_number: String,
}

async fn apply_webhook(
    pool: &PgPool,
    event_id: &str,
    event_type: &str,
    provider_payment_id: &str,
    order_id: Uuid,
    transition: WebhookTransition,
    payload_hash: [u8; 32],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let record = sqlx::query_as::<_, WebhookRecord>(
        r#"
        SELECT payment.id AS payment_id, attempt.id AS attempt_id,
               order_record.order_status, payment.status AS payment_status,
               attempt.status AS attempt_status, order_record.order_number
        FROM payment_attempts attempt
        JOIN order_payments payment ON payment.id = attempt.order_payment_id
        JOIN orders order_record ON order_record.id = payment.order_id
        WHERE attempt.provider = 'stripe' AND attempt.provider_payment_id = $1
          AND order_record.id = $2
        FOR UPDATE OF attempt, payment, order_record
        "#,
    )
    .bind(provider_payment_id)
    .bind(order_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(record) = record else {
        transaction.commit().await?;
        return Ok(());
    };
    let provider_status = match transition {
        WebhookTransition::Processing => "processing",
        WebhookTransition::Succeeded => "succeeded",
        WebhookTransition::Failed => "failed",
        WebhookTransition::Expired => "expired",
    };
    let inserted = sqlx::query(
        r#"
        INSERT INTO payment_status_events (
            id, order_payment_id, payment_attempt_id, provider, provider_event_id,
            event_type, provider_status, payload_sha256
        ) VALUES ($1, $2, $3, 'stripe', $4, $5, $6, $7)
        ON CONFLICT (provider, provider_event_id) WHERE provider_event_id IS NOT NULL DO NOTHING
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(record.payment_id)
    .bind(record.attempt_id)
    .bind(event_id)
    .bind(event_type)
    .bind(provider_status)
    .bind(payload_hash.as_slice())
    .execute(&mut *transaction)
    .await?;
    if inserted.rows_affected() == 0 || record.payment_status == "paid" {
        transaction.commit().await?;
        return Ok(());
    }
    match transition {
        WebhookTransition::Processing => {
            if matches!(record.attempt_status.as_str(), "creating" | "pending") {
                sqlx::query("UPDATE payment_attempts SET status = 'processing', updated_at = now() WHERE id = $1")
                    .bind(record.attempt_id)
                    .execute(&mut *transaction)
                    .await?;
                insert_order_event(
                    &mut transaction,
                    order_id,
                    "payment.processing",
                    "Payment processing",
                    "Stripe is processing the payment.",
                )
                .await?;
            }
        }
        WebhookTransition::Succeeded => {
            if record.order_status == "pending" && record.payment_status == "pending" {
                transition_inventory(
                    &mut transaction,
                    order_id,
                    &record.order_number,
                    InventoryPaymentTransition::Commit,
                )
                .await?;
                sqlx::query("UPDATE payment_attempts SET status = 'succeeded', updated_at = now() WHERE id = $1")
                    .bind(record.attempt_id)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("UPDATE order_payments SET status = 'paid', paid_at = now(), failure_code = NULL, failure_message = NULL, updated_at = now() WHERE id = $1")
                    .bind(record.payment_id)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("UPDATE orders SET order_status = 'confirmed', payment_status = 'paid', updated_at = now() WHERE id = $1")
                    .bind(order_id)
                    .execute(&mut *transaction)
                    .await?;
                insert_order_event(
                    &mut transaction,
                    order_id,
                    "payment.paid",
                    "Card payment received",
                    "Stripe confirmed the payment and reserved stock was committed.",
                )
                .await?;
                audit_webhook(
                    &mut transaction,
                    order_id,
                    "order.stripe_payment_succeeded",
                    event_id,
                )
                .await?;
            }
        }
        WebhookTransition::Failed | WebhookTransition::Expired => {
            if record.order_status == "pending"
                && record.payment_status == "pending"
                && !matches!(
                    record.attempt_status.as_str(),
                    "failed" | "expired" | "cancelled"
                )
            {
                transition_inventory(
                    &mut transaction,
                    order_id,
                    &record.order_number,
                    InventoryPaymentTransition::Release,
                )
                .await?;
                let (attempt_status, failure_code, title, detail, action) = match transition {
                    WebhookTransition::Expired => (
                        "expired",
                        "checkout_expired",
                        "Card checkout expired",
                        "Reserved stock was released after Stripe Checkout expired.",
                        "order.stripe_payment_expired",
                    ),
                    _ => (
                        "failed",
                        "payment_failed",
                        "Card payment failed",
                        "Reserved stock was released after Stripe reported a failed payment.",
                        "order.stripe_payment_failed",
                    ),
                };
                sqlx::query("UPDATE payment_attempts SET status = $2, failure_code = $3, failure_message = $4, updated_at = now() WHERE id = $1")
                    .bind(record.attempt_id)
                    .bind(attempt_status)
                    .bind(failure_code)
                    .bind(detail)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("UPDATE order_payments SET status = 'failed', failure_code = $2, failure_message = $3, updated_at = now() WHERE id = $1")
                    .bind(record.payment_id)
                    .bind(failure_code)
                    .bind(detail)
                    .execute(&mut *transaction)
                    .await?;
                sqlx::query("UPDATE orders SET order_status = 'cancelled', payment_status = 'failed', updated_at = now() WHERE id = $1")
                    .bind(order_id)
                    .execute(&mut *transaction)
                    .await?;
                insert_order_event(&mut transaction, order_id, "payment.failed", title, detail)
                    .await?;
                audit_webhook(&mut transaction, order_id, action, event_id).await?;
            }
        }
    }
    transaction.commit().await
}

#[derive(Clone, Copy)]
enum InventoryPaymentTransition {
    Commit,
    Release,
}

async fn transition_inventory(
    transaction: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    order_number: &str,
    operation: InventoryPaymentTransition,
) -> Result<(), sqlx::Error> {
    let lines: Vec<(Uuid, i32)> = sqlx::query_as(
        "SELECT variant_id, quantity FROM order_lines WHERE order_id = $1 ORDER BY variant_id",
    )
    .bind(order_id)
    .fetch_all(&mut **transaction)
    .await?;
    for (variant_id, quantity) in lines {
        let reason = match operation {
            InventoryPaymentTransition::Commit => {
                format!("Committed for paid order {order_number}")
            }
            InventoryPaymentTransition::Release => {
                format!("Released after unpaid order {order_number}")
            }
        };
        let result = match operation {
            InventoryPaymentTransition::Commit => {
                commit_in_transaction(transaction, variant_id, i64::from(quantity), &reason).await
            }
            InventoryPaymentTransition::Release => {
                release_in_transaction(transaction, variant_id, i64::from(quantity), &reason).await
            }
        };
        result.map_err(|error| match error {
            crate::inventory::InventoryOperationError::Database(error) => error,
            _ => sqlx::Error::Protocol(error.to_string()),
        })?;
    }
    Ok(())
}

async fn insert_order_event(
    transaction: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    event_type: &str,
    title: &str,
    detail: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO order_events (id, order_id, event_type, title, detail) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::now_v7())
    .bind(order_id)
    .bind(event_type)
    .bind(title)
    .bind(detail)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn audit_webhook(
    transaction: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    action: &str,
    event_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_log (action, entity_type, entity_id, metadata) VALUES ($1, 'order', $2, jsonb_build_object('stripe_event_id', $3::text))",
    )
    .bind(action)
    .bind(order_id.to_string())
    .bind(event_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn verify_signature(secret: &[u8], header: &str, payload: &[u8], now: i64) -> bool {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        let Some((key, value)) = part.trim().split_once('=') else {
            continue;
        };
        match key {
            "t" => timestamp = value.parse::<i64>().ok(),
            "v1" => {
                if let Ok(value) = hex::decode(value) {
                    signatures.push(value);
                }
            }
            _ => {}
        }
    }
    let Some(timestamp) = timestamp else {
        return false;
    };
    if now.abs_diff(timestamp) > WEBHOOK_TOLERANCE_SECONDS as u64 {
        return false;
    }
    let mut signed_payload = timestamp.to_string().into_bytes();
    signed_payload.push(b'.');
    signed_payload.extend_from_slice(payload);
    signatures.into_iter().any(|signature| {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret) else {
            return false;
        };
        mac.update(&signed_payload);
        mac.verify_slice(&signature).is_ok()
    })
}

pub async fn load_attempts(
    pool: &PgPool,
    payment_id: Uuid,
) -> Result<Vec<PaymentAttempt>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, attempt_number, provider, status, failure_code, failure_message,
               to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS expires_at,
               to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM payment_attempts WHERE order_payment_id = $1 ORDER BY attempt_number, id
        "#,
    )
    .bind(payment_id)
    .fetch_all(pool)
    .await
}

pub async fn load_status_events(
    pool: &PgPool,
    payment_id: Uuid,
) -> Result<Vec<PaymentStatusEvent>, sqlx::Error> {
    sqlx::query_as(
        r#"
        SELECT id, event_type, provider_status, detail,
               to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM payment_status_events WHERE order_payment_id = $1 ORDER BY created_at, id
        "#,
    )
    .bind(payment_id)
    .fetch_all(pool)
    .await
}

pub async fn cleanup_abandoned(pool: &PgPool, batch_size: i64) -> Result<u64, sqlx::Error> {
    if !(1..=1000).contains(&batch_size) {
        return Err(sqlx::Error::Protocol(
            "payment cleanup batch must be between 1 and 1000".to_owned(),
        ));
    }
    let mut transaction = pool.begin().await?;
    let records = sqlx::query_as::<_, WebhookRecord>(
        r#"
        SELECT payment.id AS payment_id, attempt.id AS attempt_id,
               order_record.order_status, payment.status AS payment_status,
               attempt.status AS attempt_status, order_record.order_number
        FROM payment_attempts attempt
        JOIN order_payments payment ON payment.id = attempt.order_payment_id
        JOIN orders order_record ON order_record.id = payment.order_id
        WHERE attempt.status IN ('creating', 'pending', 'processing')
          AND COALESCE(
              attempt.expires_at + make_interval(secs => $2),
              attempt.created_at + make_interval(secs => $2 + $3)
          ) <= now()
          AND order_record.order_status = 'pending' AND payment.status = 'pending'
        ORDER BY COALESCE(attempt.expires_at, attempt.created_at + interval '30 minutes'), attempt.id
        FOR UPDATE OF attempt, payment, order_record SKIP LOCKED
        LIMIT $1
        "#,
    )
    .bind(batch_size)
    .bind(ABANDONED_GRACE_SECONDS)
    .bind(CHECKOUT_LIFETIME_SECONDS)
    .fetch_all(&mut *transaction)
    .await?;
    for record in &records {
        let order_id: Uuid =
            sqlx::query_scalar("SELECT order_id FROM order_payments WHERE id = $1")
                .bind(record.payment_id)
                .fetch_one(&mut *transaction)
                .await?;
        transition_inventory(
            &mut transaction,
            order_id,
            &record.order_number,
            InventoryPaymentTransition::Release,
        )
        .await?;
        let detail = "Reserved stock was released after an abandoned card checkout.";
        sqlx::query("UPDATE payment_attempts SET status = 'expired', failure_code = 'checkout_abandoned', failure_message = $2, updated_at = now() WHERE id = $1")
            .bind(record.attempt_id)
            .bind(detail)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE order_payments SET status = 'failed', failure_code = 'checkout_abandoned', failure_message = $2, updated_at = now() WHERE id = $1")
            .bind(record.payment_id)
            .bind(detail)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("UPDATE orders SET order_status = 'cancelled', payment_status = 'failed', updated_at = now() WHERE id = $1")
            .bind(order_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            r#"
            INSERT INTO payment_status_events (
                id, order_payment_id, payment_attempt_id, provider,
                event_type, provider_status, detail
            ) VALUES ($1, $2, $3, 'stripe', 'payment.checkout_abandoned', 'expired', $4)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(record.payment_id)
        .bind(record.attempt_id)
        .bind(detail)
        .execute(&mut *transaction)
        .await?;
        insert_order_event(
            &mut transaction,
            order_id,
            "payment.failed",
            "Card checkout abandoned",
            detail,
        )
        .await?;
        sqlx::query("INSERT INTO audit_log (action, entity_type, entity_id) VALUES ('order.stripe_payment_abandoned', 'order', $1)")
            .bind(order_id.to_string())
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(records.len() as u64)
}

fn payment_error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}

fn invalid_webhook() -> Response {
    payment_error(
        StatusCode::BAD_REQUEST,
        "invalid_stripe_webhook",
        "The Stripe webhook could not be verified.",
    )
}

fn payments_unavailable() -> Response {
    payment_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "payments_unavailable",
        "Payments are temporarily unavailable.",
    )
}

#[cfg(test)]
mod tests {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    use super::{PaymentConfigError, PaymentService, verify_signature};
    use crate::config::Environment;

    #[test]
    fn production_requires_live_https_stripe_configuration() {
        assert_eq!(
            PaymentService::from_values(Environment::Production, None, None, None)
                .err()
                .unwrap(),
            PaymentConfigError::MissingProductionConfiguration
        );
        assert_eq!(
            PaymentService::from_values(
                Environment::Production,
                Some("sk_test_example"),
                Some("whsec_example"),
                Some("https://shop.example.com"),
            )
            .err()
            .unwrap(),
            PaymentConfigError::UnsafeProductionConfiguration
        );
    }

    #[test]
    fn webhook_signatures_require_matching_recent_raw_payloads() {
        let secret = b"whsec_integration";
        let payload = br#"{"id":"evt_test"}"#;
        let timestamp = 1_800_000_000_i64;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(format!("{timestamp}.").as_bytes());
        mac.update(payload);
        let header = format!(
            "t={timestamp},v1={}",
            hex::encode(mac.finalize().into_bytes())
        );
        assert!(verify_signature(secret, &header, payload, timestamp));
        assert!(!verify_signature(secret, &header, br#"{}"#, timestamp));
        assert!(!verify_signature(secret, &header, payload, timestamp + 301));
    }
}
