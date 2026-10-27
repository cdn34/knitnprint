use std::collections::HashSet;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

const COUNTRIES: &str = "AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CR CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG EH ER ES ET FI FJ FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TF TG TH TJ TK TL TM TN TO TR TT TV TW TZ UA UG UM US UY UZ VA VC VE VG VI VN VU WF WS YE YT ZA ZM ZW";
const MAX_AMOUNT: i64 = 10_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, FromRow)]
pub struct DeliveryRate {
    pub id: Uuid,
    pub country_code: String,
    pub region_name: String,
    pub postal_prefixes: Vec<String>,
    pub amount_minor: Option<i64>,
    pub active: bool,
}

#[derive(Serialize, ToSchema)]
pub struct DeliveryRatesSettings {
    pub enabled: bool,
    pub revision: i64,
    pub currency: String,
    pub rates: Vec<DeliveryRate>,
    pub country_codes: Vec<String>,
    pub updated_at: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateDeliveryRatesRequest {
    pub enabled: bool,
    pub revision: i64,
    pub rates: Vec<DeliveryRate>,
    pub reason: String,
}

#[derive(Deserialize, ToSchema)]
pub struct TestDeliveryRateRequest {
    pub country_code: String,
    pub postal_code: String,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct DeliveryQuote {
    pub available: bool,
    pub pricing_enabled: bool,
    pub revision: i64,
    pub currency: String,
    pub rate_id: Option<Uuid>,
    pub region_name: Option<String>,
    pub amount_minor: Option<i64>,
    pub issue_code: Option<String>,
}

#[derive(FromRow)]
struct Config {
    enabled: bool,
    currency: String,
    revision: i64,
    updated_at: String,
}

async fn config(tx: &mut Transaction<'_, Postgres>) -> Result<Config, sqlx::Error> {
    sqlx::query_as(r#"SELECT enabled,currency::text AS currency,revision,to_char(updated_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at FROM delivery_pricing_config WHERE singleton FOR SHARE"#)
        .fetch_one(&mut **tx).await
}
async fn rates(tx: &mut Transaction<'_, Postgres>) -> Result<Vec<DeliveryRate>, sqlx::Error> {
    sqlx::query_as("SELECT id,country_code::text AS country_code,region_name,postal_prefixes,amount_minor,active FROM delivery_rates ORDER BY position,id")
        .fetch_all(&mut **tx).await
}

fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    response
}
fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    no_store((status, Json(ErrorBody::new(code, message))).into_response())
}
fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "delivery_rates_unavailable",
        "Delivery rates are temporarily unavailable.",
    )
}

pub async fn load(pool: &PgPool) -> Result<DeliveryRatesSettings, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let config = config(&mut tx).await?;
    let rates = rates(&mut tx).await?;
    tx.commit().await?;
    Ok(DeliveryRatesSettings {
        enabled: config.enabled,
        revision: config.revision,
        currency: config.currency,
        rates,
        country_codes: COUNTRIES
            .split_ascii_whitespace()
            .map(str::to_owned)
            .collect(),
        updated_at: config.updated_at,
    })
}

#[utoipa::path(get,path="/api/admin/delivery-rates",tag="delivery rates",responses((status=200,body=DeliveryRatesSettings),(status=401,body=ErrorBody),(status=403,body=ErrorBody),(status=503,body=ErrorBody)))]
pub async fn get(State(state): State<AppState>, actor: AuthenticatedStaff) -> Response {
    if let Err(response) = require_capability(&actor, "settings.manage") {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match load(&pool).await {
        Ok(settings) => no_store(Json(settings).into_response()),
        Err(_) => unavailable(),
    }
}

fn validate(input: &mut UpdateDeliveryRatesRequest) -> Result<(), &'static str> {
    input.reason = input.reason.trim().to_owned();
    if input.revision < 0 || !(3..=500).contains(&input.reason.len()) || input.rates.len() > 250 {
        return Err("Provide up to 250 destinations and an audit reason of 3–500 characters.");
    }
    let mut ids = HashSet::new();
    let mut assignments = HashSet::new();
    for rate in &mut input.rates {
        rate.country_code = rate.country_code.trim().to_ascii_uppercase();
        rate.region_name = rate.region_name.trim().to_owned();
        if !COUNTRIES
            .split_ascii_whitespace()
            .any(|code| code == rate.country_code)
            || !(2..=100).contains(&rate.region_name.len())
            || !ids.insert(rate.id)
            || rate.id.is_nil()
        {
            return Err(
                "Choose a valid country, unique destination ID and region name of 2–100 characters.",
            );
        }
        if rate
            .amount_minor
            .is_some_and(|value| !(0..=MAX_AMOUNT).contains(&value))
            || (rate.active && rate.amount_minor.is_none())
        {
            return Err(
                "Every active destination needs a shipping price. Zero is an explicit free-shipping price; leave unconfigured destinations inactive.",
            );
        }
        if rate.postal_prefixes.len() > 100 {
            return Err("Use at most 100 postal prefixes per destination.");
        }
        for prefix in &mut rate.postal_prefixes {
            *prefix = prefix.trim().to_ascii_uppercase();
            if prefix.is_empty()
                || prefix.len() > 10
                || !prefix.bytes().all(|c| c.is_ascii_alphanumeric())
            {
                return Err(
                    "Postal prefixes must contain 1–10 letters or digits, without spaces or punctuation.",
                );
            }
        }
        rate.postal_prefixes.sort();
        rate.postal_prefixes.dedup();
        let prefixes = if rate.postal_prefixes.is_empty() {
            vec![String::new()]
        } else {
            rate.postal_prefixes.clone()
        };
        for prefix in prefixes {
            if !assignments.insert((rate.country_code.clone(), prefix)) {
                return Err(
                    "Two destinations in the same country cannot share a postal prefix or both cover the whole country.",
                );
            }
        }
    }
    if input.enabled
        && !input
            .rates
            .iter()
            .any(|rate| rate.active && rate.amount_minor.is_some())
    {
        return Err("Configure and activate at least one destination before enabling the table.");
    }
    Ok(())
}

#[utoipa::path(put,path="/api/admin/delivery-rates",tag="delivery rates",request_body=UpdateDeliveryRatesRequest,responses((status=200,body=DeliveryRatesSettings),(status=401,body=ErrorBody),(status=403,body=ErrorBody),(status=409,body=ErrorBody),(status=422,body=ErrorBody),(status=503,body=ErrorBody)))]
pub async fn update(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Json(mut input): Json<UpdateDeliveryRatesRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, "settings.manage") {
        return response.into_response();
    }
    if let Err(message) = validate(&mut input) {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_delivery_rates",
            message,
        );
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let revision = sqlx::query_scalar::<_, i64>(
        "SELECT revision FROM delivery_pricing_config WHERE singleton FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await;
    match revision {
        Ok(revision) if revision == input.revision => {}
        Ok(_) => {
            return error(
                StatusCode::CONFLICT,
                "delivery_rates_conflict",
                "Another administrator changed this table. Reload the saved version before trying again.",
            );
        }
        Err(_) => return unavailable(),
    }
    if sqlx::query("DELETE FROM delivery_rates")
        .execute(&mut *tx)
        .await
        .is_err()
    {
        return unavailable();
    }
    for (position, rate) in input.rates.iter().enumerate() {
        if sqlx::query("INSERT INTO delivery_rates (id,country_code,region_name,postal_prefixes,amount_minor,active,position) VALUES ($1,$2,$3,$4,$5,$6,$7)")
            .bind(rate.id).bind(&rate.country_code).bind(&rate.region_name).bind(&rate.postal_prefixes).bind(rate.amount_minor).bind(rate.active).bind(position as i32)
            .execute(&mut *tx).await.is_err() { return unavailable() }
    }
    let snapshot = serde_json::json!({"enabled":input.enabled,"rates":input.rates,"revision":input.revision+1});
    if sqlx::query("UPDATE delivery_pricing_config SET enabled=$1,revision=revision+1,updated_at=now(),updated_by_staff_user_id=$2 WHERE singleton")
        .bind(input.enabled).bind(actor.id).execute(&mut *tx).await.is_err()
        || sqlx::query("INSERT INTO settings_history (id,actor_staff_user_id,reason,snapshot) VALUES ($1,$2,$3,$4)")
            .bind(Uuid::now_v7()).bind(actor.id).bind(&input.reason).bind(&snapshot).execute(&mut *tx).await.is_err()
        || sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,reason) VALUES ($1,'delivery_rates.update','delivery_pricing','store',$2)")
            .bind(actor.id).bind(&input.reason).execute(&mut *tx).await.is_err()
        || tx.commit().await.is_err() { return unavailable() }
    match load(&pool).await {
        Ok(settings) => no_store(Json(settings).into_response()),
        Err(_) => unavailable(),
    }
}

/// PT/ES formats follow UPU postal addressing specifications; PT island prefixes
/// follow CTT coverage: Madeira 90–94, Azores 95–99 (see docs/operations/delivery-rates.md).
fn normalize_postal(country: &str, raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_uppercase();
    if value.is_empty()
        || value.len() > 20
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b' ' || c == b'-')
    {
        return None;
    }
    let normalized = value.replace([' ', '-'], "");
    let valid = match country {
        "PT" => {
            normalized.len() == 7
                && normalized.bytes().all(|c| c.is_ascii_digit())
                && !normalized.starts_with('0')
        }
        "ES" => {
            normalized.len() == 5
                && normalized.bytes().all(|c| c.is_ascii_digit())
                && normalized[..2]
                    .parse::<u8>()
                    .is_ok_and(|p| (1..=52).contains(&p))
        }
        _ => !normalized.is_empty() && normalized.len() <= 12,
    };
    valid.then_some(normalized)
}

pub fn evaluate(
    enabled: bool,
    revision: i64,
    currency: &str,
    rates: &[DeliveryRate],
    country: &str,
    postal: &str,
) -> DeliveryQuote {
    let mut quote = DeliveryQuote {
        available: false,
        pricing_enabled: enabled,
        revision,
        currency: currency.into(),
        rate_id: None,
        region_name: None,
        amount_minor: None,
        issue_code: None,
    };
    if !enabled {
        quote.available = true;
        quote.amount_minor = Some(0);
        return quote;
    }
    let country = country.trim().to_ascii_uppercase();
    let Some(postal) = normalize_postal(&country, postal) else {
        quote.issue_code = Some("shipping_invalid_postal_code".into());
        return quote;
    };
    let matched = rates
        .iter()
        .filter(|rate| rate.country_code == country)
        .filter_map(|rate| {
            let specificity = if rate.postal_prefixes.is_empty() {
                Some(0)
            } else {
                rate.postal_prefixes
                    .iter()
                    .filter(|prefix| postal.starts_with(prefix.as_str()))
                    .map(String::len)
                    .max()
            }?;
            Some((specificity, rate))
        })
        .max_by_key(|(specificity, _)| *specificity)
        .map(|(_, rate)| rate);
    match matched {
        Some(rate) => {
            quote.rate_id = Some(rate.id);
            quote.region_name = Some(rate.region_name.clone());
            if rate.active && rate.amount_minor.is_some() {
                quote.available = true;
                quote.amount_minor = rate.amount_minor;
            } else {
                quote.issue_code = Some("shipping_destination_unavailable".into());
            }
        }
        None => quote.issue_code = Some("shipping_destination_unavailable".into()),
    }
    quote
}

pub async fn quote_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    currency: &str,
    country: &str,
    postal: &str,
) -> Result<DeliveryQuote, sqlx::Error> {
    let config = config(tx).await?;
    let store_currency: String =
        sqlx::query_scalar("SELECT currency::text FROM store_settings WHERE singleton FOR SHARE")
            .fetch_one(&mut **tx)
            .await?;
    let rates = if config.enabled {
        rates(tx).await?
    } else {
        Vec::new()
    };
    let mut quote = evaluate(
        config.enabled,
        config.revision,
        if config.enabled {
            &config.currency
        } else {
            currency
        },
        &rates,
        country,
        postal,
    );
    if config.enabled && (currency != store_currency || currency != config.currency) {
        quote.available = false;
        quote.amount_minor = None;
        quote.issue_code = Some("shipping_currency_unavailable".into());
    }
    Ok(quote)
}

#[utoipa::path(post,path="/api/admin/delivery-rates/test",tag="delivery rates",request_body=TestDeliveryRateRequest,responses((status=200,body=DeliveryQuote),(status=401,body=ErrorBody),(status=403,body=ErrorBody),(status=422,body=ErrorBody),(status=503,body=ErrorBody)))]
pub async fn test_destination(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Json(input): Json<TestDeliveryRateRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, "settings.manage") {
        return response.into_response();
    }
    let country = input.country_code.trim().to_ascii_uppercase();
    if !COUNTRIES
        .split_ascii_whitespace()
        .any(|code| code == country)
        || input.postal_code.len() > 20
    {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_test_destination",
            "Choose a country and a valid postal code.",
        );
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let settings = match load(&pool).await {
        Ok(settings) => settings,
        Err(_) => return unavailable(),
    };
    let mut quote = evaluate(
        true,
        settings.revision,
        &settings.currency,
        &settings.rates,
        &country,
        &input.postal_code,
    );
    quote.pricing_enabled = settings.enabled;
    no_store(Json(quote).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rate(name: &str, prefixes: &[&str], amount: Option<i64>, active: bool) -> DeliveryRate {
        DeliveryRate {
            id: Uuid::new_v4(),
            country_code: "PT".into(),
            region_name: name.into(),
            postal_prefixes: prefixes.iter().map(|s| s.to_string()).collect(),
            amount_minor: amount,
            active,
        }
    }
    #[test]
    fn regions_are_automatic_specific_and_do_not_fall_back_to_cheaper_country_rates() {
        let rates = vec![
            rate("Country", &[], Some(100), true),
            rate("Madeira", &["90", "91", "92", "93", "94"], Some(800), true),
            rate("Azores", &["95", "96", "97", "98", "99"], None, false),
        ];
        assert_eq!(
            evaluate(true, 1, "EUR", &rates, "PT", "4000-123").amount_minor,
            Some(100)
        );
        assert_eq!(
            evaluate(true, 1, "EUR", &rates, "PT", "9400-123").amount_minor,
            Some(800)
        );
        assert!(!evaluate(true, 1, "EUR", &rates, "PT", "9500-123").available);
        assert!(!evaluate(true, 1, "EUR", &rates, "PT", "9999-123").available);
        assert!(!evaluate(true, 1, "EUR", &rates, "FR", "75001").available);
        assert!(!evaluate(true, 1, "EUR", &rates, "PT", "9000").available);
        assert_eq!(
            evaluate(false, 1, "EUR", &rates, "PT", "9000").amount_minor,
            Some(0)
        );
        assert_eq!(normalize_postal("PT", " 9000123 "), Some("9000123".into()));
        assert!(normalize_postal("ES", "28001").is_some());
        assert!(normalize_postal("ES", "99001").is_none());
    }
    #[test]
    fn unpriced_active_and_ambiguous_rules_are_rejected() {
        let mut request = UpdateDeliveryRatesRequest {
            enabled: false,
            revision: 0,
            rates: vec![rate("Madeira", &["90"], None, true)],
            reason: "Testing validation".into(),
        };
        assert!(validate(&mut request).is_err());
        request.rates[0].active = false;
        assert!(validate(&mut request).is_ok());
        request.rates[0].amount_minor = Some(0);
        request.rates[0].active = true;
        request.enabled = true;
        assert!(validate(&mut request).is_ok());
        request
            .rates
            .push(rate("Duplicate", &["90"], Some(200), true));
        assert!(validate(&mut request).is_err());
        request.rates.pop();
        request
            .rates
            .push(rate("Specific", &["9000"], Some(200), true));
        assert!(validate(&mut request).is_ok());
    }
}
