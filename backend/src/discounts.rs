use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
};

const DISCOUNTS_MANAGE: &str = "discounts.manage";

#[derive(Deserialize, ToSchema)]
pub struct CreateDiscountRequest {
    pub code: String,
    pub kind: String,
    pub value: i64,
    pub currency: String,
    #[serde(default)]
    pub minimum_order_minor: i64,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub usage_limit: Option<i64>,
    pub per_customer_limit: Option<i64>,
    pub reason: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ChangeDiscountStatusRequest {
    pub enabled: bool,
    pub reason: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateDiscountRequest {
    pub code: String,
    pub kind: String,
    pub value: i64,
    pub currency: String,
    #[serde(default)]
    pub minimum_order_minor: i64,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub usage_limit: Option<i64>,
    pub per_customer_limit: Option<i64>,
    pub reason: String,
}

impl From<UpdateDiscountRequest> for CreateDiscountRequest {
    fn from(input: UpdateDiscountRequest) -> Self {
        Self {
            code: input.code,
            kind: input.kind,
            value: input.value,
            currency: input.currency,
            minimum_order_minor: input.minimum_order_minor,
            starts_at: input.starts_at,
            ends_at: input.ends_at,
            usage_limit: input.usage_limit,
            per_customer_limit: input.per_customer_limit,
            reason: input.reason,
        }
    }
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct Discount {
    pub id: Uuid,
    pub code: String,
    pub kind: String,
    pub value: i64,
    pub currency: String,
    pub minimum_order_minor: i64,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub usage_limit: Option<i64>,
    pub per_customer_limit: Option<i64>,
    pub usage_count: i64,
    pub status: String,
    pub created_at: String,
}

#[derive(Clone, Serialize, ToSchema)]
pub struct AppliedDiscount {
    pub code: String,
    pub kind: String,
    pub amount_minor: i64,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct OrderDiscount {
    pub code: String,
    pub kind: String,
    pub value: i64,
    pub amount_minor: i64,
    pub currency: String,
}

#[derive(FromRow)]
struct DiscountRule {
    id: Uuid,
    code: String,
    kind: String,
    fixed_amount_minor: Option<i64>,
    percentage_basis_points: Option<i32>,
    currency: String,
    minimum_order_minor: i64,
    available_now: bool,
    usage_limit: Option<i64>,
    per_customer_limit: Option<i64>,
}

pub struct EvaluatedDiscount {
    pub id: Uuid,
    pub code: String,
    pub kind: String,
    pub fixed_amount_minor: Option<i64>,
    pub percentage_basis_points: Option<i32>,
    pub amount_minor: i64,
    pub currency: String,
}

#[derive(Debug)]
pub enum EvaluationError {
    Unavailable,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for EvaluationError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/discounts",
    tag = "admin discounts",
    responses(
        (status = 200, body = [Discount]),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn list(State(state): State<AppState>, actor: AuthenticatedStaff) -> Response {
    if let Err(response) = require_capability(&actor, DISCOUNTS_MANAGE) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    if expire_due(&pool).await.is_err() {
        return unavailable();
    }
    match discount_records(&pool).await {
        Ok(records) => Json(records).into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/discounts",
    tag = "admin discounts",
    request_body = CreateDiscountRequest,
    responses(
        (status = 201, body = Discount),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Json(input): Json<CreateDiscountRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, DISCOUNTS_MANAGE) {
        return response.into_response();
    }
    let Ok(input) = ValidatedDiscount::new(input) else {
        return invalid();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let id = Uuid::now_v7();
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let inserted = sqlx::query(
        r#"
        INSERT INTO discounts (
            id, code, kind, fixed_amount_minor, percentage_basis_points, currency,
            minimum_order_minor, starts_at, ends_at, usage_limit, per_customer_limit,
            created_by_staff_user_id
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8::timestamptz,$9::timestamptz,$10,$11,$12)
        ON CONFLICT (code) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(&input.code)
    .bind(&input.kind)
    .bind(input.fixed_amount_minor)
    .bind(input.percentage_basis_points)
    .bind(&input.currency)
    .bind(input.minimum_order_minor)
    .bind(input.starts_at.as_deref())
    .bind(input.ends_at.as_deref())
    .bind(input.usage_limit)
    .bind(input.per_customer_limit)
    .bind(actor.id)
    .execute(&mut *tx)
    .await;
    let Ok(inserted) = inserted else {
        return unavailable();
    };
    if inserted.rows_affected() == 0 {
        return error(
            StatusCode::CONFLICT,
            "discount_code_exists",
            "That discount code already exists.",
        );
    }
    if sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,reason,metadata) VALUES ($1,'discount.create','discount',$2,$3,jsonb_build_object('code',$4::text))")
        .bind(actor.id).bind(id.to_string()).bind(&input.reason).bind(&input.code).execute(&mut *tx).await.is_err()
        || tx.commit().await.is_err()
    { return unavailable() }
    match discount_record(&pool, id).await {
        Ok(Some(record)) => (StatusCode::CREATED, Json(record)).into_response(),
        _ => unavailable(),
    }
}

#[utoipa::path(
    put,
    path = "/api/admin/discounts/{discount_id}",
    tag = "admin discounts",
    params(("discount_id" = Uuid, Path)),
    request_body = UpdateDiscountRequest,
    responses(
        (status = 200, body = Discount),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn update(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(discount_id): Path<Uuid>,
    Json(input): Json<UpdateDiscountRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, DISCOUNTS_MANAGE) {
        return response.into_response();
    }
    let Ok(input) = ValidatedDiscount::new(input.into()) else {
        return invalid();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let changed = sqlx::query(
        r#"UPDATE discounts SET
            code = $2, kind = $3, fixed_amount_minor = $4,
            percentage_basis_points = $5, currency = $6,
            minimum_order_minor = $7, starts_at = $8::timestamptz,
            ends_at = $9::timestamptz, usage_limit = $10,
            per_customer_limit = $11, updated_at = now()
        WHERE id = $1"#,
    )
    .bind(discount_id)
    .bind(&input.code)
    .bind(&input.kind)
    .bind(input.fixed_amount_minor)
    .bind(input.percentage_basis_points)
    .bind(&input.currency)
    .bind(input.minimum_order_minor)
    .bind(input.starts_at.as_deref())
    .bind(input.ends_at.as_deref())
    .bind(input.usage_limit)
    .bind(input.per_customer_limit)
    .execute(&mut *tx)
    .await;
    let changed = match changed {
        Ok(changed) => changed,
        Err(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("23505") =>
        {
            return error(
                StatusCode::CONFLICT,
                "discount_code_exists",
                "That discount code already exists.",
            );
        }
        Err(_) => return unavailable(),
    };
    if changed.rows_affected() == 0 {
        return error(
            StatusCode::NOT_FOUND,
            "discount_not_found",
            "The discount was not found.",
        );
    }
    if sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,reason,metadata) VALUES ($1,'discount.update','discount',$2,$3,jsonb_build_object('code',$4::text))")
        .bind(actor.id).bind(discount_id.to_string()).bind(&input.reason).bind(&input.code).execute(&mut *tx).await.is_err()
        || tx.commit().await.is_err()
    { return unavailable() }
    if expire_due(&pool).await.is_err() {
        return unavailable();
    }
    match discount_record(&pool, discount_id).await {
        Ok(Some(record)) => Json(record).into_response(),
        Ok(None) => error(
            StatusCode::NOT_FOUND,
            "discount_not_found",
            "The discount was not found.",
        ),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/discounts/{discount_id}/status",
    tag = "admin discounts",
    params(("discount_id" = Uuid, Path)),
    request_body = ChangeDiscountStatusRequest,
    responses(
        (status = 200, body = Discount),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn change_status(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(discount_id): Path<Uuid>,
    Json(input): Json<ChangeDiscountStatusRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, DISCOUNTS_MANAGE) {
        return response.into_response();
    }
    let reason = input.reason.trim();
    if !(3..=500).contains(&reason.len()) {
        return invalid();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let status = if input.enabled { "active" } else { "disabled" };
    let changed = sqlx::query(
        r#"UPDATE discounts SET status = $2,
             disabled_by_staff_user_id = CASE WHEN $2 = 'disabled' THEN $3 ELSE NULL END,
             disabled_reason = CASE WHEN $2 = 'disabled' THEN $4 ELSE NULL END,
             updated_at = now()
           WHERE id = $1
             AND ($2 = 'disabled' OR ends_at IS NULL OR ends_at > now())"#,
    )
    .bind(discount_id)
    .bind(status)
    .bind(actor.id)
    .bind(reason)
    .execute(&mut *tx)
    .await;
    let Ok(changed) = changed else {
        return unavailable();
    };
    if changed.rows_affected() == 0 {
        let exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM discounts WHERE id = $1)")
                .bind(discount_id)
                .fetch_one(&mut *tx)
                .await;
        if matches!(exists, Ok(true)) && input.enabled {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "discount_expired",
                "An expired discount cannot be enabled until its end date is edited.",
            );
        }
        return error(
            StatusCode::NOT_FOUND,
            "discount_not_found",
            "The discount was not found.",
        );
    }
    let action = if input.enabled {
        "discount.enable"
    } else {
        "discount.disable"
    };
    if sqlx::query("INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,reason) VALUES ($1,$2,'discount',$3,$4)")
        .bind(actor.id).bind(action).bind(discount_id.to_string()).bind(reason).execute(&mut *tx).await.is_err()
        || tx.commit().await.is_err()
    { return unavailable() }
    match discount_record(&pool, discount_id).await {
        Ok(Some(record)) => Json(record).into_response(),
        Ok(None) => error(
            StatusCode::NOT_FOUND,
            "discount_not_found",
            "The discount was not found.",
        ),
        Err(_) => unavailable(),
    }
}

struct ValidatedDiscount {
    code: String,
    kind: String,
    fixed_amount_minor: Option<i64>,
    percentage_basis_points: Option<i32>,
    currency: String,
    minimum_order_minor: i64,
    starts_at: Option<String>,
    ends_at: Option<String>,
    usage_limit: Option<i64>,
    per_customer_limit: Option<i64>,
    reason: String,
}

impl ValidatedDiscount {
    fn new(input: CreateDiscountRequest) -> Result<Self, ()> {
        let code = normalize_code(&input.code).ok_or(())?;
        let kind = input.kind.trim().to_ascii_lowercase();
        let currency = input.currency.trim().to_ascii_uppercase();
        let reason = input.reason.trim().to_owned();
        let fixed_amount_minor = (kind == "fixed").then_some(input.value);
        let percentage_basis_points = if kind == "percentage" {
            i32::try_from(input.value).ok()
        } else {
            None
        };
        let dates_valid = [input.starts_at.as_deref(), input.ends_at.as_deref()]
            .into_iter()
            .flatten()
            .all(|value| {
                OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).is_ok()
            });
        if !matches!(kind.as_str(), "fixed" | "percentage")
            || fixed_amount_minor.is_some_and(|value| value <= 0)
            || percentage_basis_points.is_some_and(|value| !(1..=10_000).contains(&value))
            || currency.len() != 3
            || !currency.bytes().all(|byte| byte.is_ascii_uppercase())
            || input.minimum_order_minor < 0
            || input.usage_limit.is_some_and(|value| value <= 0)
            || input.per_customer_limit.is_some_and(|value| value <= 0)
            || !(3..=500).contains(&reason.len())
            || !dates_valid
        {
            return Err(());
        }
        if let (Some(start), Some(end)) = (&input.starts_at, &input.ends_at) {
            let start =
                OffsetDateTime::parse(start, &time::format_description::well_known::Rfc3339)
                    .map_err(|_| ())?;
            let end = OffsetDateTime::parse(end, &time::format_description::well_known::Rfc3339)
                .map_err(|_| ())?;
            if start >= end {
                return Err(());
            }
        }
        Ok(Self {
            code,
            kind,
            fixed_amount_minor,
            percentage_basis_points,
            currency,
            minimum_order_minor: input.minimum_order_minor,
            starts_at: input.starts_at,
            ends_at: input.ends_at,
            usage_limit: input.usage_limit,
            per_customer_limit: input.per_customer_limit,
            reason,
        })
    }
}

pub fn normalize_code(code: &str) -> Option<String> {
    let code = code.trim().to_ascii_uppercase();
    ((3..=32).contains(&code.len())
        && code.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_uppercase()
                || byte.is_ascii_digit()
                || (index > 0 && matches!(byte, b'_' | b'-'))
        }))
    .then_some(code)
}

pub async fn find_by_code_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM discounts WHERE code = $1")
        .bind(code)
        .fetch_optional(&mut **tx)
        .await
}

pub async fn evaluate(
    pool: &PgPool,
    discount_id: Uuid,
    subtotal_minor: i64,
    currency: &str,
    customer_id: Option<Uuid>,
) -> Result<EvaluatedDiscount, EvaluationError> {
    let rule = sqlx::query_as::<_, DiscountRule>(&rule_query(false))
        .bind(discount_id)
        .fetch_optional(pool)
        .await?
        .ok_or(EvaluationError::Unavailable)?;
    evaluate_rule(pool, &rule, subtotal_minor, currency, customer_id).await
}

pub async fn evaluate_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    discount_id: Uuid,
    subtotal_minor: i64,
    currency: &str,
    customer_id: Option<Uuid>,
) -> Result<EvaluatedDiscount, EvaluationError> {
    let rule = sqlx::query_as::<_, DiscountRule>(&rule_query(true))
        .bind(discount_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(EvaluationError::Unavailable)?;
    let global_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM discount_usages WHERE discount_id = $1")
            .bind(rule.id)
            .fetch_one(&mut **tx)
            .await?;
    let customer_count = if let Some(customer_id) = customer_id {
        sqlx::query_scalar(
            "SELECT count(*) FROM discount_usages WHERE discount_id = $1 AND customer_id = $2",
        )
        .bind(rule.id)
        .bind(customer_id)
        .fetch_one(&mut **tx)
        .await?
    } else {
        0
    };
    calculate(
        &rule,
        subtotal_minor,
        currency,
        global_count,
        customer_count,
    )
}

async fn evaluate_rule(
    pool: &PgPool,
    rule: &DiscountRule,
    subtotal_minor: i64,
    currency: &str,
    customer_id: Option<Uuid>,
) -> Result<EvaluatedDiscount, EvaluationError> {
    let global_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM discount_usages WHERE discount_id = $1")
            .bind(rule.id)
            .fetch_one(pool)
            .await?;
    let customer_count = if let Some(customer_id) = customer_id {
        sqlx::query_scalar(
            "SELECT count(*) FROM discount_usages WHERE discount_id = $1 AND customer_id = $2",
        )
        .bind(rule.id)
        .bind(customer_id)
        .fetch_one(pool)
        .await?
    } else {
        0
    };
    calculate(rule, subtotal_minor, currency, global_count, customer_count)
}

const AUTOMATIC_EXPIRY_REASON: &str = "Automatically disabled after the end date.";

pub async fn expire_due(pool: &PgPool) -> Result<u64, sqlx::Error> {
    expire_due_with_executor(pool).await
}

async fn expire_due_with_executor<'e, E>(executor: E) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let result = sqlx::query(
        r#"WITH expired AS (
            UPDATE discounts
               SET status = 'disabled',
                   disabled_by_staff_user_id = NULL,
                   disabled_reason = $1,
                   updated_at = now()
             WHERE status = 'active'
               AND ends_at IS NOT NULL
               AND ends_at <= now()
         RETURNING id, code, ends_at
        )
        INSERT INTO audit_log (action, entity_type, entity_id, reason, metadata)
        SELECT 'discount.expire', 'discount', id::text, $1,
               jsonb_build_object('code', code, 'ends_at', ends_at)
          FROM expired"#,
    )
    .bind(AUTOMATIC_EXPIRY_REASON)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

fn calculate(
    rule: &DiscountRule,
    subtotal_minor: i64,
    currency: &str,
    global_count: i64,
    customer_count: i64,
) -> Result<EvaluatedDiscount, EvaluationError> {
    if !rule.available_now
        || rule.currency != currency
        || subtotal_minor < rule.minimum_order_minor
        || rule.usage_limit.is_some_and(|limit| global_count >= limit)
        || rule
            .per_customer_limit
            .is_some_and(|limit| customer_count >= limit)
    {
        return Err(EvaluationError::Unavailable);
    }
    let amount_minor = if let Some(fixed) = rule.fixed_amount_minor {
        fixed.min(subtotal_minor)
    } else {
        subtotal_minor
            .checked_mul(i64::from(
                rule.percentage_basis_points
                    .ok_or(EvaluationError::Unavailable)?,
            ))
            .and_then(|value| value.checked_div(10_000))
            .ok_or(EvaluationError::Unavailable)?
    };
    if amount_minor <= 0 {
        return Err(EvaluationError::Unavailable);
    }
    Ok(EvaluatedDiscount {
        id: rule.id,
        code: rule.code.clone(),
        kind: rule.kind.clone(),
        fixed_amount_minor: rule.fixed_amount_minor,
        percentage_basis_points: rule.percentage_basis_points,
        amount_minor,
        currency: rule.currency.clone(),
    })
}

fn rule_query(lock: bool) -> String {
    format!(
        r#"SELECT id,code,kind,fixed_amount_minor,percentage_basis_points,
        currency::text AS currency,minimum_order_minor,
        status = 'active' AND (starts_at IS NULL OR starts_at <= now())
          AND (ends_at IS NULL OR ends_at > now()) AS available_now,
        usage_limit,per_customer_limit FROM discounts WHERE id = $1{}"#,
        if lock { " FOR UPDATE" } else { "" }
    )
}

pub async fn record_usage(
    tx: &mut Transaction<'_, Postgres>,
    discount: &EvaluatedDiscount,
    order_id: Uuid,
    customer_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO order_discounts (order_id,discount_id,code,kind,fixed_amount_minor,percentage_basis_points,amount_minor,currency) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(order_id).bind(discount.id).bind(&discount.code).bind(&discount.kind)
        .bind(discount.fixed_amount_minor).bind(discount.percentage_basis_points)
        .bind(discount.amount_minor).bind(&discount.currency).execute(&mut **tx).await?;
    sqlx::query(
        "INSERT INTO discount_usages (id,discount_id,order_id,customer_id) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::now_v7())
    .bind(discount.id)
    .bind(order_id)
    .bind(customer_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn load_order_discount(
    pool: &PgPool,
    order_id: Uuid,
) -> Result<Option<OrderDiscount>, sqlx::Error> {
    sqlx::query_as("SELECT code,kind,CASE WHEN kind = 'fixed' THEN fixed_amount_minor ELSE percentage_basis_points::bigint END AS value,amount_minor,currency::text AS currency FROM order_discounts WHERE order_id = $1")
        .bind(order_id).fetch_optional(pool).await
}

fn discount_select() -> &'static str {
    r#"SELECT discount.id,discount.code,discount.kind,
        CASE WHEN discount.kind = 'fixed' THEN discount.fixed_amount_minor ELSE discount.percentage_basis_points::bigint END AS value,
        discount.currency::text AS currency,discount.minimum_order_minor,
        CASE WHEN discount.starts_at IS NULL THEN NULL ELSE to_char(discount.starts_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') END AS starts_at,
        CASE WHEN discount.ends_at IS NULL THEN NULL ELSE to_char(discount.ends_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') END AS ends_at,
        discount.usage_limit,discount.per_customer_limit,
        (SELECT count(*) FROM discount_usages usage WHERE usage.discount_id = discount.id) AS usage_count,
        discount.status,to_char(discount.created_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM discounts discount"#
}

async fn discount_records(pool: &PgPool) -> Result<Vec<Discount>, sqlx::Error> {
    sqlx::query_as(&format!(
        "{} ORDER BY created_at DESC,id DESC LIMIT 200",
        discount_select()
    ))
    .fetch_all(pool)
    .await
}

async fn discount_record(pool: &PgPool, id: Uuid) -> Result<Option<Discount>, sqlx::Error> {
    sqlx::query_as(&format!("{} WHERE discount.id = $1", discount_select()))
        .bind(id)
        .fetch_optional(pool)
        .await
}

fn invalid() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_discount",
        "Provide a valid code, value, currency, dates, limits, and audit reason.",
    )
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "discounts_unavailable",
        "Discounts are temporarily unavailable.",
    )
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}
