use std::collections::HashSet;

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
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
    pub shipping_zones: Vec<ShippingZone>,
    pub tax_rules: Vec<TaxRule>,
    pub integrations: IntegrationHealth,
    pub history: Vec<SettingsHistoryRecord>,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct ShippingZone {
    pub id: Uuid,
    pub name: String,
    pub country_codes: Vec<String>,
    pub active: bool,
    pub methods: Vec<ShippingMethod>,
}

#[derive(Serialize, ToSchema)]
pub struct ShippingMethod {
    pub id: Uuid,
    pub name: String,
    pub flat_rate_minor: i64,
    pub currency: String,
    pub active: bool,
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
    pub packlink: crate::packlink::PacklinkConfigurationStatus,
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
    pub shipping_zones: Vec<ShippingZoneInput>,
    pub tax_rules: Vec<TaxRuleInput>,
    pub reason: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ShippingZoneInput {
    pub name: String,
    pub country_codes: Vec<String>,
    pub active: bool,
    pub methods: Vec<ShippingMethodInput>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ShippingMethodInput {
    pub name: String,
    pub flat_rate_minor: i64,
    pub active: bool,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct TaxRuleInput {
    pub name: String,
    pub country_codes: Vec<String>,
    pub rate_basis_points: i32,
    pub active: bool,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct ShippingSelection {
    pub id: Uuid,
    pub zone_name: String,
    pub method_name: String,
    pub amount_minor: i64,
    pub currency: String,
    pub provider: String,
    pub carrier_name: String,
    pub service_id: String,
    pub departure_dropoff: bool,
    pub destination_dropoff: bool,
    pub transit_hours: i32,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct TaxSelection {
    pub rule_name: String,
    pub rate_basis_points: i32,
    pub taxable_amount_minor: i64,
    pub amount_minor: i64,
    pub behavior: String,
}

#[derive(Clone)]
pub struct CommercialPricing {
    pub shipping_methods: Vec<ShippingSelection>,
    pub shipping: ShippingSelection,
    pub tax: TaxSelection,
    pub tax_rule_id: Option<Uuid>,
    pub country_code: String,
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

#[derive(Debug)]
pub enum PricingError {
    Unavailable,
    Database(sqlx::Error),
}

#[derive(FromRow)]
struct SettingsRow {
    store_name: String,
    support_email: String,
    currency: String,
    tax_enabled: bool,
    updated_at: String,
}

#[derive(FromRow)]
struct ZoneRow {
    id: Uuid,
    name: String,
    country_codes: Vec<String>,
    active: bool,
}

#[derive(FromRow)]
struct MethodRow {
    id: Uuid,
    shipping_zone_id: Uuid,
    name: String,
    flat_rate_minor: i64,
    currency: String,
    active: bool,
}

#[derive(FromRow)]
struct PricingSettingsRow {
    currency: String,
}

#[derive(FromRow)]
struct PricingZoneRow {
    id: Uuid,
    name: String,
}

#[derive(FromRow)]
struct PricingMethodRow {
    id: Uuid,
    name: String,
    flat_rate_minor: i64,
    currency: String,
}

#[derive(FromRow)]
struct PricingTaxRow {
    id: Uuid,
    name: String,
    rate_basis_points: i32,
}

#[derive(FromRow)]
struct PacklinkQuoteRow {
    id: Uuid,
    service_id: String,
    carrier_name: String,
    service_name: String,
    amount_minor: i64,
    currency: String,
    departure_dropoff: bool,
    destination_dropoff: bool,
    transit_hours: i32,
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
        || sqlx::query("DELETE FROM shipping_zones")
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
    for (zone_position, zone) in input.shipping_zones.iter().enumerate() {
        let zone_id = Uuid::now_v7();
        if sqlx::query("INSERT INTO shipping_zones (id,name,country_codes,priority,active) VALUES ($1,$2,$3,$4,$5)")
            .bind(zone_id).bind(&zone.name).bind(&zone.country_codes)
            .bind(i32::try_from(zone_position).unwrap_or(i32::MAX)).bind(zone.active)
            .execute(&mut *tx).await.is_err()
        { return unavailable() }
        for (method_position, method) in zone.methods.iter().enumerate() {
            if sqlx::query("INSERT INTO shipping_methods (id,shipping_zone_id,name,flat_rate_minor,currency,position,active) VALUES ($1,$2,$3,$4,$5,$6,$7)")
                .bind(Uuid::now_v7()).bind(zone_id).bind(&method.name)
                .bind(method.flat_rate_minor).bind(&input.currency)
                .bind(i32::try_from(method_position).unwrap_or(i32::MAX)).bind(method.active)
                .execute(&mut *tx).await.is_err()
            { return unavailable() }
        }
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
    let zone_rows = sqlx::query_as::<_, ZoneRow>(
        "SELECT id,name,country_codes,active FROM shipping_zones ORDER BY priority,id",
    )
    .fetch_all(pool)
    .await?;
    let method_rows = sqlx::query_as::<_, MethodRow>(
        "SELECT id,shipping_zone_id,name,flat_rate_minor,currency::text AS currency,active FROM shipping_methods ORDER BY shipping_zone_id,position,id",
    )
    .fetch_all(pool)
    .await?;
    let shipping_zones = zone_rows
        .into_iter()
        .map(|zone| ShippingZone {
            id: zone.id,
            name: zone.name,
            country_codes: zone.country_codes,
            active: zone.active,
            methods: method_rows
                .iter()
                .filter(|method| method.shipping_zone_id == zone.id)
                .map(|method| ShippingMethod {
                    id: method.id,
                    name: method.name.clone(),
                    flat_rate_minor: method.flat_rate_minor,
                    currency: method.currency.clone(),
                    active: method.active,
                })
                .collect(),
        })
        .collect();
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
        shipping_zones,
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
            packlink: state.packlink.status(),
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
    shipping_zones: Vec<ShippingZoneInput>,
    tax_rules: Vec<TaxRuleInput>,
    reason: String,
}

impl ValidatedSettings {
    fn new(mut input: UpdateCommercialSettingsRequest) -> Result<Self, ()> {
        input.store_name = input.store_name.trim().to_owned();
        input.support_email = input.support_email.trim().to_ascii_lowercase();
        input.currency = input.currency.trim().to_ascii_uppercase();
        input.reason = input.reason.trim().to_owned();
        for zone in &mut input.shipping_zones {
            zone.name = zone.name.trim().to_owned();
            normalize_countries(&mut zone.country_codes)?;
            for method in &mut zone.methods {
                method.name = method.name.trim().to_owned();
            }
        }
        for rule in &mut input.tax_rules {
            rule.name = rule.name.trim().to_owned();
            normalize_countries(&mut rule.country_codes)?;
        }
        if !(2..=100).contains(&input.store_name.len())
            || !valid_email(&input.support_email)
            || input.currency.len() != 3
            || !input
                .currency
                .bytes()
                .all(|value| value.is_ascii_uppercase())
            || !(3..=500).contains(&input.reason.len())
            || input.shipping_zones.is_empty()
            || input.shipping_zones.len() > 50
            || input.tax_rules.len() > 100
            || input.shipping_zones.iter().any(|zone| {
                !(2..=100).contains(&zone.name.len())
                    || zone.methods.is_empty()
                    || zone.methods.len() > 20
                    || zone.methods.iter().any(|method| {
                        !(2..=100).contains(&method.name.len()) || method.flat_rate_minor < 0
                    })
            })
            || input.tax_rules.iter().any(|rule| {
                !(2..=100).contains(&rule.name.len())
                    || !(0..=10_000).contains(&rule.rate_basis_points)
            })
            || overlaps(
                &input
                    .shipping_zones
                    .iter()
                    .filter(|zone| zone.active)
                    .map(|zone| &zone.country_codes)
                    .collect::<Vec<_>>(),
            )
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
            shipping_zones: input.shipping_zones,
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

pub async fn evaluate(
    pool: &PgPool,
    currency: &str,
    country_code: &str,
    selected_method_id: Option<Uuid>,
    merchandise_minor: i64,
) -> Result<CommercialPricing, PricingError> {
    let mut tx = pool.begin().await.map_err(PricingError::Database)?;
    let pricing = evaluate_in_transaction(
        &mut tx,
        currency,
        country_code,
        selected_method_id,
        merchandise_minor,
    )
    .await?;
    tx.commit().await.map_err(PricingError::Database)?;
    Ok(pricing)
}

pub async fn evaluate_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    currency: &str,
    country_code: &str,
    selected_method_id: Option<Uuid>,
    merchandise_minor: i64,
) -> Result<CommercialPricing, PricingError> {
    let settings = sqlx::query_as::<_, PricingSettingsRow>(
        "SELECT currency::text AS currency FROM store_settings WHERE singleton FOR SHARE",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(PricingError::Database)?;
    if settings.currency != currency || merchandise_minor < 0 {
        return Err(PricingError::Unavailable);
    }
    let zone = sqlx::query_as::<_, PricingZoneRow>(
        "SELECT id,name FROM shipping_zones WHERE active AND (cardinality(country_codes)=0 OR $1=ANY(country_codes)) ORDER BY (cardinality(country_codes)=0),priority,id LIMIT 1 FOR SHARE",
    )
    .bind(country_code)
    .fetch_optional(&mut **tx)
    .await
    .map_err(PricingError::Database)?
    .ok_or(PricingError::Unavailable)?;
    let method_rows = sqlx::query_as::<_, PricingMethodRow>(
        "SELECT id,name,flat_rate_minor,currency::text AS currency FROM shipping_methods WHERE shipping_zone_id=$1 AND active ORDER BY position,id FOR SHARE",
    )
    .bind(zone.id)
    .fetch_all(&mut **tx)
    .await
    .map_err(PricingError::Database)?;
    let shipping_methods: Vec<_> = method_rows
        .into_iter()
        .filter(|method| method.currency == currency)
        .map(|method| ShippingSelection {
            id: method.id,
            zone_name: zone.name.clone(),
            method_name: method.name,
            amount_minor: method.flat_rate_minor,
            currency: method.currency,
            provider: "manual".into(),
            carrier_name: String::new(),
            service_id: String::new(),
            departure_dropoff: false,
            destination_dropoff: false,
            transit_hours: 0,
        })
        .collect();
    let shipping = selected_method_id
        .and_then(|id| {
            shipping_methods
                .iter()
                .find(|method| method.id == id)
                .cloned()
        })
        .or_else(|| shipping_methods.first().cloned())
        .ok_or(PricingError::Unavailable)?;
    pricing_for_shipping(
        tx,
        country_code,
        merchandise_minor,
        shipping_methods,
        shipping,
    )
    .await
}

pub async fn evaluate_packlink_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    cart_id: Uuid,
    currency: &str,
    country_code: &str,
    selected_quote_id: Option<Uuid>,
    merchandise_minor: i64,
    expected_request_hash: Option<&[u8]>,
) -> Result<CommercialPricing, PricingError> {
    let settings_currency: String =
        sqlx::query_scalar("SELECT currency::text FROM store_settings WHERE singleton FOR SHARE")
            .fetch_one(&mut **tx)
            .await
            .map_err(PricingError::Database)?;
    if settings_currency != currency || merchandise_minor < 0 {
        return Err(PricingError::Unavailable);
    }
    let rows = sqlx::query_as::<_, PacklinkQuoteRow>(
        r#"SELECT id,service_id,carrier_name,service_name,amount_minor,
        currency::text AS currency,departure_dropoff,destination_dropoff,transit_hours
        FROM cart_shipping_quotes
        WHERE cart_id=$1 AND expires_at>now() AND currency=$2
          AND ($3::bytea IS NULL OR request_hash=$3)
        ORDER BY amount_minor,transit_hours,id FOR SHARE"#,
    )
    .bind(cart_id)
    .bind(currency)
    .bind(expected_request_hash)
    .fetch_all(&mut **tx)
    .await
    .map_err(PricingError::Database)?;
    let shipping_methods = rows
        .into_iter()
        .map(|quote| ShippingSelection {
            id: quote.id,
            zone_name: "Packlink PRO".into(),
            method_name: packlink_method_name(
                &quote.carrier_name,
                &quote.service_name,
                quote.departure_dropoff,
            ),
            amount_minor: quote.amount_minor,
            currency: quote.currency,
            provider: "packlink".into(),
            carrier_name: quote.carrier_name,
            service_id: quote.service_id,
            departure_dropoff: quote.departure_dropoff,
            destination_dropoff: quote.destination_dropoff,
            transit_hours: quote.transit_hours,
        })
        .collect::<Vec<_>>();
    let shipping_methods = customer_packlink_choices(shipping_methods);
    let shipping = selected_quote_id
        .and_then(|id| {
            shipping_methods
                .iter()
                .find(|method| method.id == id)
                .cloned()
        })
        .or_else(|| shipping_methods.first().cloned())
        .ok_or(PricingError::Unavailable)?;
    pricing_for_shipping(
        tx,
        country_code,
        merchandise_minor,
        shipping_methods,
        shipping,
    )
    .await
}

fn customer_packlink_choices(methods: Vec<ShippingSelection>) -> Vec<ShippingSelection> {
    let Some(cheapest) = methods.first().cloned() else {
        return Vec::new();
    };
    let fastest = methods
        .iter()
        .filter(|method| method.transit_hours > 0)
        .min_by_key(|method| (method.transit_hours, method.amount_minor))
        .cloned();
    let mut choices = vec![cheapest];
    if let Some(fastest) = fastest
        && choices.iter().all(|choice| choice.id != fastest.id)
    {
        choices.push(fastest);
    }
    choices
}

pub async fn evaluate_packlink(
    pool: &PgPool,
    cart_id: Uuid,
    currency: &str,
    country_code: &str,
    selected_quote_id: Option<Uuid>,
    merchandise_minor: i64,
) -> Result<CommercialPricing, PricingError> {
    let mut tx = pool.begin().await.map_err(PricingError::Database)?;
    let pricing = evaluate_packlink_in_transaction(
        &mut tx,
        cart_id,
        currency,
        country_code,
        selected_quote_id,
        merchandise_minor,
        None,
    )
    .await?;
    tx.commit().await.map_err(PricingError::Database)?;
    Ok(pricing)
}

async fn pricing_for_shipping(
    tx: &mut Transaction<'_, Postgres>,
    country_code: &str,
    merchandise_minor: i64,
    shipping_methods: Vec<ShippingSelection>,
    shipping: ShippingSelection,
) -> Result<CommercialPricing, PricingError> {
    let tax_enabled: bool =
        sqlx::query_scalar("SELECT tax_enabled FROM store_settings WHERE singleton FOR SHARE")
            .fetch_one(&mut **tx)
            .await
            .map_err(PricingError::Database)?;
    let taxable_amount_minor = merchandise_minor
        .checked_add(shipping.amount_minor)
        .ok_or(PricingError::Unavailable)?;
    let tax_rule = if tax_enabled {
        Some(sqlx::query_as::<_, PricingTaxRow>(
            "SELECT id,name,rate_basis_points FROM tax_rules WHERE active AND (cardinality(country_codes)=0 OR $1=ANY(country_codes)) ORDER BY (cardinality(country_codes)=0),priority,id LIMIT 1 FOR SHARE",
        )
        .bind(country_code)
        .fetch_optional(&mut **tx)
        .await
        .map_err(PricingError::Database)?
        .ok_or(PricingError::Unavailable)?)
    } else {
        None
    };
    let (tax_rule_id, rule_name, rate_basis_points, behavior) = match tax_rule {
        Some(rule) => (
            Some(rule.id),
            rule.name,
            rule.rate_basis_points,
            "exclusive",
        ),
        None => (None, "Tax calculation disabled".into(), 0, "disabled"),
    };
    let amount_minor =
        i64::try_from(i128::from(taxable_amount_minor) * i128::from(rate_basis_points) / 10_000)
            .map_err(|_| PricingError::Unavailable)?;
    Ok(CommercialPricing {
        shipping_methods,
        shipping,
        tax: TaxSelection {
            rule_name,
            rate_basis_points,
            taxable_amount_minor,
            amount_minor,
            behavior: behavior.into(),
        },
        tax_rule_id,
        country_code: country_code.into(),
    })
}

fn packlink_method_name(carrier: &str, service: &str, departure_dropoff: bool) -> String {
    if departure_dropoff {
        format!("{carrier} · {service} · entrega no ponto pelo remetente")
    } else {
        format!("{carrier} · {service} · recolha em Anadia")
    }
}

pub async fn record_order_snapshots(
    tx: &mut Transaction<'_, Postgres>,
    order_id: Uuid,
    pricing: &CommercialPricing,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO order_shipping_snapshots (order_id,shipping_method_id,zone_name,method_name,country_code,amount_minor,currency,provider,carrier_name,external_service_id,departure_dropoff,destination_dropoff,transit_hours) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
        .bind(order_id).bind(pricing.shipping.id).bind(&pricing.shipping.zone_name)
        .bind(&pricing.shipping.method_name).bind(&pricing.country_code)
        .bind(pricing.shipping.amount_minor).bind(&pricing.shipping.currency)
        .bind(&pricing.shipping.provider).bind(&pricing.shipping.carrier_name)
        .bind(&pricing.shipping.service_id).bind(pricing.shipping.departure_dropoff)
        .bind(pricing.shipping.destination_dropoff).bind(pricing.shipping.transit_hours)
        .execute(&mut **tx).await?;
    sqlx::query("INSERT INTO order_tax_snapshots (order_id,tax_rule_id,rule_name,country_code,rate_basis_points,taxable_amount_minor,amount_minor,behavior) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(order_id).bind(pricing.tax_rule_id).bind(&pricing.tax.rule_name)
        .bind(&pricing.country_code).bind(pricing.tax.rate_basis_points)
        .bind(pricing.tax.taxable_amount_minor).bind(pricing.tax.amount_minor)
        .bind(&pricing.tax.behavior).execute(&mut **tx).await?;
    Ok(())
}

pub async fn load_order_shipping(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<OrderShipping, sqlx::Error> {
    sqlx::query_as("SELECT zone_name,method_name,country_code::text AS country_code,amount_minor,currency::text AS currency,provider,carrier_name,external_service_id,departure_dropoff,destination_dropoff,transit_hours FROM order_shipping_snapshots WHERE order_id=$1")
        .bind(order_id).fetch_one(pool).await
}

pub async fn load_order_tax(pool: &PgPool, order_id: Uuid) -> Result<OrderTax, sqlx::Error> {
    sqlx::query_as("SELECT rule_name,country_code::text AS country_code,rate_basis_points,taxable_amount_minor,amount_minor,behavior FROM order_tax_snapshots WHERE order_id=$1")
        .bind(order_id).fetch_one(pool).await
}

fn invalid() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_settings",
        "Provide valid store identity, non-overlapping zones, methods, tax rules, and an audit reason.",
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
