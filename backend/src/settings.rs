use std::collections::HashSet;

use axum::{
    Json,
    extract::State,
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
    customers::valid_email,
    error::ErrorBody,
};

const SETTINGS_MANAGE: &str = "settings.manage";

#[derive(Serialize, ToSchema)]
pub struct CommercialSettings {
    pub store_name: String,
    pub support_email: String,
    pub currency: String,
    pub tax_enabled: bool,
    pub checkout_suspended: bool,
    pub tax_rules: Vec<TaxRule>,
    pub integrations: IntegrationHealth,
    pub history: Vec<SettingsHistoryRecord>,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct TaxRule {
    pub id: Uuid,
    pub name: String,
    pub country_codes: Vec<String>,
    pub rate_basis_points: i32,
    pub active: bool,
}

#[derive(Serialize, ToSchema)]
pub struct IntegrationHealth {
    pub database: String,
    pub media_storage: String,
    pub email: String,
    pub payments: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct SettingsHistoryRecord {
    pub id: Uuid,
    pub reason: String,
    pub actor_display_name: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct UpdateCommercialSettingsRequest {
    pub store_name: String,
    pub support_email: String,
    pub currency: String,
    pub tax_enabled: bool,
    pub tax_rules: Vec<TaxRuleInput>,
    pub reason: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct TaxRuleInput {
    pub name: String,
    pub country_codes: Vec<String>,
    pub rate_basis_points: i32,
    pub active: bool,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct TaxSelection {
    pub rule_name: String,
    pub rate_basis_points: i32,
    pub taxable_amount_minor: i64,
    pub amount_minor: i64,
    pub behavior: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct OrderShipping {
    pub zone_name: String,
    pub method_name: String,
    pub country_code: String,
    pub amount_minor: i64,
    pub currency: String,
    pub provider: String,
    pub carrier_name: String,
    pub external_service_id: String,
    pub departure_dropoff: bool,
    pub destination_dropoff: bool,
    pub transit_hours: i32,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct OrderTax {
    pub rule_name: String,
    pub country_code: String,
    pub rate_basis_points: i32,
    pub taxable_amount_minor: i64,
    pub amount_minor: i64,
    pub behavior: String,
}

#[derive(FromRow)]
struct SettingsRow {
    store_name: String,
    support_email: String,
    currency: String,
    tax_enabled: bool,
    updated_at: String,
}

#[utoipa::path(
    get,
    path = "/api/admin/settings",
    tag = "admin settings",
    responses(
        (status = 200, body = CommercialSettings),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn get(State(state): State<AppState>, actor: AuthenticatedStaff) -> Response {
    if let Err(response) = require_capability(&actor, SETTINGS_MANAGE) {
        return response.into_response();
    }
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    match load(pool, &state).await {
        Ok(settings) => Json(settings).into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/settings",
    tag = "admin settings",
    request_body = UpdateCommercialSettingsRequest,
    responses(
        (status = 200, body = CommercialSettings),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn update(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Json(input): Json<UpdateCommercialSettingsRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, SETTINGS_MANAGE) {
        return response.into_response();
    }
    let Ok(input) = ValidatedSettings::new(input) else {
        return invalid();
    };
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    if sqlx::query("SELECT singleton FROM store_settings WHERE singleton FOR UPDATE")
        .fetch_one(&mut *tx)
        .await
        .is_err()
    {
        return unavailable();
    }
    if sqlx::query(
        "UPDATE store_settings SET store_name=$1,support_email=$2,currency=$3,tax_enabled=$4,updated_by_staff_user_id=$5,updated_at=now() WHERE singleton",
    )
    .bind(&input.store_name)
    .bind(&input.support_email)
    .bind(&input.currency)
    .bind(input.tax_enabled)
    .bind(actor.id)
    .execute(&mut *tx)
    .await
    .is_err()
        || sqlx::query("DELETE FROM tax_rules")
            .execute(&mut *tx)
            .await
            .is_err()
    {
        return unavailable();
    }
    for (priority, rule) in input.tax_rules.iter().enumerate() {
        if sqlx::query("INSERT INTO tax_rules (id,name,country_codes,rate_basis_points,priority,active) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(Uuid::now_v7()).bind(&rule.name).bind(&rule.country_codes)
            .bind(rule.rate_basis_points).bind(i32::try_from(priority).unwrap_or(i32::MAX)).bind(rule.active)
            .execute(&mut *tx).await.is_err()
        { return unavailable() }
    }
    let snapshot = match serde_json::to_value(&input) {
        Ok(snapshot) => snapshot,
        Err(_) => return unavailable(),
    };
    if sqlx::query("INSERT INTO settings_history (id,actor_staff_user_id,reason,snapshot) VALUES ($1,$2,$3,$4)")
        .bind(Uuid::now_v7()).bind(actor.id).bind(&input.reason).bind(snapshot)
        .execute(&mut *tx).await.is_err()
        || sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,reason) VALUES ($1,'settings.update','store_settings','store',$2)")
            .bind(actor.id).bind(&input.reason).execute(&mut *tx).await.is_err()
        || tx.commit().await.is_err()
    { return unavailable() }
    match load(pool, &state).await {
        Ok(settings) => Json(settings).into_response(),
        Err(_) => unavailable(),
    }
}

async fn load(pool: &PgPool, state: &AppState) -> Result<CommercialSettings, sqlx::Error> {
    let settings = sqlx::query_as::<_, SettingsRow>(
        r#"SELECT store_name,support_email,currency::text AS currency,tax_enabled,
        to_char(updated_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
        FROM store_settings WHERE singleton"#,
    )
    .fetch_one(pool)
    .await?;
    let tax_rules = sqlx::query_as::<_, TaxRule>(
        "SELECT id,name,country_codes,rate_basis_points,active FROM tax_rules ORDER BY priority,id",
    )
    .fetch_all(pool)
    .await?;
    let history = sqlx::query_as::<_, SettingsHistoryRecord>(
        r#"SELECT history.id,history.reason,staff.display_name AS actor_display_name,
        to_char(history.created_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM settings_history history LEFT JOIN staff_users staff ON staff.id=history.actor_staff_user_id
        ORDER BY history.created_at DESC,history.id DESC LIMIT 20"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(CommercialSettings {
        store_name: settings.store_name,
        support_email: settings.support_email,
        currency: settings.currency,
        tax_enabled: settings.tax_enabled,
        checkout_suspended: crate::checkout::SUSPENDED,
        tax_rules,
        integrations: IntegrationHealth {
            database: "configured".into(),
            media_storage: if state.media_storage.is_some() {
                "configured"
            } else {
                "unavailable"
            }
            .into(),
            email: state.email.status().into(),
            payments: if state.payments.enabled() {
                "stripe_configured"
            } else if state.manual_payments_enabled {
                "manual_development"
            } else {
                "unavailable"
            }
            .into(),
        },
        history,
        updated_at: settings.updated_at,
    })
}

#[derive(Serialize)]
struct ValidatedSettings {
    store_name: String,
    support_email: String,
    currency: String,
    tax_enabled: bool,
    tax_rules: Vec<TaxRuleInput>,
    reason: String,
}

impl ValidatedSettings {
    fn new(mut input: UpdateCommercialSettingsRequest) -> Result<Self, ()> {
        input.store_name = input.store_name.trim().to_owned();
        input.support_email = input.support_email.trim().to_ascii_lowercase();
        input.currency = input.currency.trim().to_ascii_uppercase();
        input.reason = input.reason.trim().to_owned();
        for rule in &mut input.tax_rules {
            rule.name = rule.name.trim().to_owned();
            normalize_countries(&mut rule.country_codes)?;
        }
        if !(2..=100).contains(&input.store_name.len())
            || !valid_email(&input.support_email)
            || input.currency != crate::CURRENCY
            || !(3..=500).contains(&input.reason.len())
            || input.tax_rules.len() > 100
            || input.tax_rules.iter().any(|rule| {
                !(2..=100).contains(&rule.name.len())
                    || !(0..=10_000).contains(&rule.rate_basis_points)
            })
            || overlaps(
                &input
                    .tax_rules
                    .iter()
                    .filter(|rule| rule.active)
                    .map(|rule| &rule.country_codes)
                    .collect::<Vec<_>>(),
            )
        {
            return Err(());
        }
        Ok(Self {
            store_name: input.store_name,
            support_email: input.support_email,
            currency: input.currency,
            tax_enabled: input.tax_enabled,
            tax_rules: input.tax_rules,
            reason: input.reason,
        })
    }
}

fn normalize_countries(countries: &mut Vec<String>) -> Result<(), ()> {
    for country in countries.iter_mut() {
        *country = country.trim().to_ascii_uppercase();
        if country.len() != 2 || !country.bytes().all(|value| value.is_ascii_uppercase()) {
            return Err(());
        }
    }
    countries.sort();
    countries.dedup();
    if countries.len() > 249 {
        Err(())
    } else {
        Ok(())
    }
}

fn overlaps(groups: &[&Vec<String>]) -> bool {
    let mut countries = HashSet::new();
    let mut fallback = false;
    for group in groups {
        if group.is_empty() {
            if fallback {
                return true;
            }
            fallback = true;
        } else if group.iter().any(|country| !countries.insert(country)) {
            return true;
        }
    }
    false
}

pub async fn load_order_shipping(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<OrderShipping, sqlx::Error> {
    sqlx::query_as("SELECT zone_name,method_name,country_code::text AS country_code,amount_minor,currency::text AS currency,provider,carrier_name,external_service_id,departure_dropoff,destination_dropoff,transit_hours FROM order_shipping_snapshots WHERE order_id=$1")
        .bind(order_id).fetch_one(pool).await
}

pub(crate) enum CheckoutTaxError {
    Unavailable,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for CheckoutTaxError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

pub(crate) async fn checkout_tax(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    country: &str,
    merchandise_minor: i64,
) -> Result<OrderTax, CheckoutTaxError> {
    let enabled: bool =
        sqlx::query_scalar("SELECT tax_enabled FROM store_settings WHERE singleton FOR SHARE")
            .fetch_one(&mut **tx)
            .await?;
    let rule: Option<(String, i32)> = if enabled {
        sqlx::query_as("SELECT name,rate_basis_points FROM tax_rules WHERE active AND (cardinality(country_codes)=0 OR $1=ANY(country_codes)) ORDER BY (cardinality(country_codes)=0),priority,id LIMIT 1 FOR SHARE")
            .bind(country).fetch_optional(&mut **tx).await?
    } else {
        None
    };
    let matched = rule.is_some();
    if enabled && !matched {
        return Err(CheckoutTaxError::Unavailable);
    }
    let (rule_name, rate) = rule.unwrap_or_else(|| ("No additional tax".into(), 0));
    let amount = i64::try_from(i128::from(merchandise_minor) * i128::from(rate) / 10000)
        .map_err(|_| sqlx::Error::Protocol("tax total overflow".into()))?;
    Ok(OrderTax {
        rule_name,
        country_code: country.into(),
        rate_basis_points: rate,
        taxable_amount_minor: merchandise_minor,
        amount_minor: amount,
        behavior: if !enabled {
            "disabled"
        } else if matched {
            "exclusive"
        } else {
            "no_matching_rule"
        }
        .into(),
    })
}

pub(crate) async fn record_checkout_snapshots(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order_id: Uuid,
    currency: &str,
    country: &str,
    shipping: &crate::delivery_rates::DeliveryQuote,
    tax: &OrderTax,
) -> Result<(), sqlx::Error> {
    let zone = shipping
        .region_name
        .as_deref()
        .unwrap_or("Delivery pricing pending");
    let method = if shipping.pricing_enabled {
        "Destination flat rate"
    } else {
        "Shipping not charged"
    };
    sqlx::query("INSERT INTO order_shipping_snapshots (order_id,shipping_method_id,zone_name,method_name,country_code,amount_minor,currency) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(order_id).bind(shipping.rate_id).bind(zone).bind(method).bind(country).bind(shipping.amount_minor.unwrap_or(0)).bind(currency).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO order_tax_snapshots (order_id,rule_name,country_code,rate_basis_points,taxable_amount_minor,amount_minor,behavior) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(order_id).bind(&tax.rule_name).bind(country).bind(tax.rate_basis_points)
        .bind(tax.taxable_amount_minor).bind(tax.amount_minor).bind(&tax.behavior).execute(&mut **tx).await?;
    Ok(())
}

pub async fn load_order_tax(pool: &PgPool, order_id: Uuid) -> Result<OrderTax, sqlx::Error> {
    sqlx::query_as("SELECT rule_name,country_code::text AS country_code,rate_basis_points,taxable_amount_minor,amount_minor,behavior FROM order_tax_snapshots WHERE order_id=$1")
        .bind(order_id).fetch_one(pool).await
}

fn invalid() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_settings",
        "Provide valid store identity, EUR currency, non-overlapping tax rules, and an audit reason.",
    )
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "settings_unavailable",
        "Commercial settings are temporarily unavailable.",
    )
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}
