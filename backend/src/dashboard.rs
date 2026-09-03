use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::AuthenticatedStaff,
    error::ErrorBody,
    settings::{TaxAutomationStatus, tax_automation_status},
};

const ORDERS_READ: &str = "orders.read";
const INVENTORY_ADJUST: &str = "inventory.adjust";
const SETTINGS_MANAGE: &str = "settings.manage";

#[derive(Serialize, ToSchema)]
pub struct OperationalDashboard {
    pub generated_at: String,
    pub timezone: String,
    pub period_start: String,
    pub currency: String,
    pub access: DashboardAccess,
    pub metrics: DashboardMetrics,
    pub tax_automation: Option<TaxAutomationStatus>,
    pub definitions: Vec<MetricDefinition>,
    pub paid_awaiting_fulfillment: Vec<DashboardOrder>,
    pub recent_orders: Vec<DashboardOrder>,
    pub low_stock_variants: Vec<DashboardInventory>,
    pub failed_payments: Vec<DashboardFailedPayment>,
    pub recent_refunds: Vec<DashboardRefund>,
}

#[derive(Serialize, ToSchema)]
pub struct DashboardAccess {
    pub orders: bool,
    pub inventory: bool,
    pub settings: bool,
}

#[derive(Serialize, ToSchema, Default)]
pub struct DashboardMetrics {
    pub orders_total: Option<i64>,
    pub orders_today: Option<i64>,
    pub gross_revenue_minor: Option<i64>,
    pub refunds_minor: Option<i64>,
    pub net_revenue_minor: Option<i64>,
    pub paid_awaiting_fulfillment: Option<i64>,
    pub failed_payments: Option<i64>,
    pub low_stock_variants: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct MetricDefinition {
    pub key: String,
    pub description: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct DashboardOrder {
    pub id: Uuid,
    pub order_number: String,
    pub customer_name: String,
    pub payment_status: String,
    pub fulfillment_status: String,
    pub total_minor: i64,
    pub currency: String,
    pub created_at: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct DashboardInventory {
    pub variant_id: Uuid,
    pub product_title: String,
    pub variant_title: String,
    pub sku: String,
    pub available_quantity: i64,
    pub low_stock_threshold: i64,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct DashboardFailedPayment {
    pub order_id: Uuid,
    pub order_number: String,
    pub customer_name: String,
    pub amount_minor: i64,
    pub currency: String,
    pub failure_message: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct DashboardRefund {
    pub id: Uuid,
    pub order_id: Uuid,
    pub order_number: String,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(FromRow)]
struct OrderMetrics {
    orders_total: i64,
    orders_today: i64,
    gross_revenue_minor: i64,
    refunds_minor: i64,
    paid_awaiting_fulfillment: i64,
    failed_payments: i64,
}

#[derive(FromRow)]
struct DashboardClock {
    generated_at: String,
    period_start: String,
    currency: String,
}

#[utoipa::path(
    get,
    path = "/api/admin/dashboard",
    tag = "admin dashboard",
    responses(
        (status = 200, body = OperationalDashboard),
        (status = 401, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn get(State(state): State<AppState>, actor: AuthenticatedStaff) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    let access = DashboardAccess {
        orders: actor.capabilities.iter().any(|value| value == ORDERS_READ),
        inventory: actor
            .capabilities
            .iter()
            .any(|value| value == INVENTORY_ADJUST),
        settings: actor
            .capabilities
            .iter()
            .any(|value| value == SETTINGS_MANAGE),
    };
    match load(&pool, access).await {
        Ok(dashboard) => no_store(Json(dashboard).into_response()),
        Err(_) => unavailable(),
    }
}

async fn load(pool: &PgPool, access: DashboardAccess) -> Result<OperationalDashboard, sqlx::Error> {
    let clock = sqlx::query_as::<_, DashboardClock>(
        r#"SELECT
        to_char(now() AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS generated_at,
        to_char(date_trunc('day',now() AT TIME ZONE 'UTC'),'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS period_start,
        currency::text AS currency
        FROM store_settings WHERE singleton"#,
    )
    .fetch_one(pool)
    .await?;

    let mut metrics = DashboardMetrics::default();
    let mut paid_awaiting_fulfillment = Vec::new();
    let mut recent_orders = Vec::new();
    let mut failed_payments = Vec::new();
    let mut recent_refunds = Vec::new();
    let tax_automation = if access.settings {
        Some(tax_automation_status(pool).await?)
    } else {
        None
    };
    if access.orders {
        let order_metrics = sqlx::query_as::<_, OrderMetrics>(
            r#"SELECT
            (SELECT count(*)::bigint FROM orders) AS orders_total,
            (SELECT count(*)::bigint FROM orders WHERE created_at >= date_trunc('day',now() AT TIME ZONE 'UTC') AT TIME ZONE 'UTC') AS orders_today,
            (SELECT COALESCE(sum(amount_minor),0)::bigint FROM order_payments WHERE paid_at IS NOT NULL AND currency=$1) AS gross_revenue_minor,
            (SELECT COALESCE(sum(amount_minor),0)::bigint FROM order_refunds WHERE status='succeeded' AND currency=$1) AS refunds_minor,
            (SELECT count(*)::bigint FROM orders WHERE payment_status IN ('paid','partially_refunded') AND fulfillment_status <> 'fulfilled' AND order_status <> 'cancelled') AS paid_awaiting_fulfillment,
            (SELECT count(*)::bigint FROM order_payments WHERE status='failed') AS failed_payments"#,
        )
        .bind(&clock.currency)
        .fetch_one(pool)
        .await?;
        metrics.orders_total = Some(order_metrics.orders_total);
        metrics.orders_today = Some(order_metrics.orders_today);
        metrics.gross_revenue_minor = Some(order_metrics.gross_revenue_minor);
        metrics.refunds_minor = Some(order_metrics.refunds_minor);
        metrics.net_revenue_minor = Some(
            order_metrics
                .gross_revenue_minor
                .saturating_sub(order_metrics.refunds_minor),
        );
        metrics.paid_awaiting_fulfillment = Some(order_metrics.paid_awaiting_fulfillment);
        metrics.failed_payments = Some(order_metrics.failed_payments);

        paid_awaiting_fulfillment = order_list(
            pool,
            "WHERE payment_status IN ('paid','partially_refunded') AND fulfillment_status <> 'fulfilled' AND order_status <> 'cancelled' ORDER BY created_at,id LIMIT 8",
        )
        .await?;
        recent_orders = order_list(pool, "ORDER BY created_at DESC,id DESC LIMIT 8").await?;
        failed_payments = sqlx::query_as::<_, DashboardFailedPayment>(
            r#"SELECT order_record.id AS order_id,order_record.order_number,
            btrim(order_record.customer_first_name || ' ' || order_record.customer_last_name) AS customer_name,
            payment.amount_minor,payment.currency::text AS currency,
            COALESCE(payment.failure_message,'Payment failed') AS failure_message,
            to_char(payment.updated_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
            FROM order_payments payment JOIN orders order_record ON order_record.id=payment.order_id
            WHERE payment.status='failed' ORDER BY payment.updated_at DESC,payment.id DESC LIMIT 8"#,
        )
        .fetch_all(pool)
        .await?;
        recent_refunds = sqlx::query_as::<_, DashboardRefund>(
            r#"SELECT refund.id,refund.order_id,order_record.order_number,refund.amount_minor,
            refund.currency::text AS currency,refund.status,refund.reason,
            to_char(refund.created_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
            FROM order_refunds refund JOIN orders order_record ON order_record.id=refund.order_id
            ORDER BY refund.created_at DESC,refund.id DESC LIMIT 8"#,
        )
        .fetch_all(pool)
        .await?;
    }

    let mut low_stock_variants = Vec::new();
    if access.inventory {
        metrics.low_stock_variants = Some(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*)::bigint FROM inventory_items WHERE available_quantity <= low_stock_threshold",
            )
            .fetch_one(pool)
            .await?,
        );
        low_stock_variants = sqlx::query_as::<_, DashboardInventory>(
            r#"SELECT inventory.variant_id,product.title AS product_title,
            variant.title AS variant_title,variant.sku,inventory.available_quantity,
            inventory.low_stock_threshold FROM inventory_items inventory
            JOIN product_variants variant ON variant.id=inventory.variant_id
            JOIN products product ON product.id=variant.product_id
            WHERE inventory.available_quantity <= inventory.low_stock_threshold
            ORDER BY inventory.available_quantity,product.title,variant.title,inventory.variant_id LIMIT 8"#,
        )
        .fetch_all(pool)
        .await?;
    }

    Ok(OperationalDashboard {
        generated_at: clock.generated_at,
        timezone: "UTC".into(),
        period_start: clock.period_start,
        currency: clock.currency,
        access,
        metrics,
        tax_automation,
        definitions: metric_definitions(),
        paid_awaiting_fulfillment,
        recent_orders,
        low_stock_variants,
        failed_payments,
        recent_refunds,
    })
}

async fn order_list(pool: &PgPool, clause: &str) -> Result<Vec<DashboardOrder>, sqlx::Error> {
    let query = format!(
        r#"SELECT id,order_number,
        btrim(customer_first_name || ' ' || customer_last_name) AS customer_name,
        payment_status,fulfillment_status,total_minor,currency::text AS currency,
        to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM orders {clause}"#
    );
    sqlx::query_as(&query).fetch_all(pool).await
}

fn metric_definitions() -> Vec<MetricDefinition> {
    [
        ("orders_total", "All orders in every state and currency."),
        ("orders_today", "Orders created since 00:00 UTC today."),
        (
            "gross_revenue_minor",
            "Captured payments in the configured store currency, before refunds.",
        ),
        (
            "refunds_minor",
            "Succeeded refunds in the configured store currency.",
        ),
        (
            "net_revenue_minor",
            "Captured payments minus succeeded refunds in the configured store currency.",
        ),
        (
            "paid_awaiting_fulfillment",
            "Paid or partially refunded, non-cancelled orders not yet fully fulfilled.",
        ),
        (
            "failed_payments",
            "Orders whose current payment record is failed.",
        ),
        (
            "low_stock_variants",
            "Variants whose available quantity is at or below their configured threshold.",
        ),
    ]
    .into_iter()
    .map(|(key, description)| MetricDefinition {
        key: key.into(),
        description: description.into(),
    })
    .collect()
}

fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    response
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody::new(
            "dashboard_unavailable",
            "Operational dashboard data is temporarily unavailable.",
        )),
    )
        .into_response()
}
