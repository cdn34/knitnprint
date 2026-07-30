use std::collections::BTreeSet;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, hash_password, require_capability},
    error::ErrorBody,
};

const STAFF_MANAGE: &str = "staff.manage";

#[derive(Serialize, ToSchema, FromRow)]
pub struct StaffRecord {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub disabled: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateStaffRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct DisableStaffRequest {
    pub reason: String,
}

#[utoipa::path(
    get,
    path = "/api/admin/staff",
    tag = "staff",
    responses(
        (status = 200, description = "Staff records", body = [StaffRecord]),
        (status = 401, description = "Authentication required", body = ErrorBody),
        (status = 403, description = "Capability required", body = ErrorBody)
    )
)]
pub async fn list(State(state): State<AppState>, actor: AuthenticatedStaff) -> Response {
    if let Err(response) = require_capability(&actor, STAFF_MANAGE) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };

    match sqlx::query_as::<_, StaffRecord>(
        r#"
        SELECT
            u.id,
            u.email::text AS email,
            u.display_name,
            u.role,
            COALESCE(
                array_agg(sc.capability_name ORDER BY sc.capability_name)
                    FILTER (WHERE sc.capability_name IS NOT NULL),
                ARRAY[]::text[]
            ) AS capabilities,
            u.disabled_at IS NOT NULL AS disabled
        FROM staff_users u
        LEFT JOIN staff_capabilities sc ON sc.staff_user_id = u.id
        GROUP BY u.id
        ORDER BY u.created_at, u.id
        "#,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(staff) => Json(staff).into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/staff",
    tag = "staff",
    request_body = CreateStaffRequest,
    responses(
        (status = 201, description = "Staff member created", body = StaffRecord),
        (status = 401, description = "Authentication required", body = ErrorBody),
        (status = 403, description = "Capability required", body = ErrorBody),
        (status = 409, description = "Email already exists", body = ErrorBody),
        (status = 422, description = "Invalid staff details", body = ErrorBody)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Json(input): Json<CreateStaffRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, STAFF_MANAGE) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let email = input.email.trim().to_lowercase();
    let display_name = input.display_name.trim();
    if email.is_empty()
        || email.len() > 254
        || display_name.is_empty()
        || display_name.len() > 120
        || input.password.len() < 12
        || input.password.len() > 256
    {
        return invalid_input();
    }

    let capabilities: Vec<String> = input
        .capabilities
        .into_iter()
        .map(|capability| capability.trim().to_owned())
        .filter(|capability| !capability.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let known_capabilities = match sqlx::query_scalar::<_, String>(
        "SELECT name FROM capabilities WHERE name = ANY($1) ORDER BY name",
    )
    .bind(&capabilities)
    .fetch_all(&pool)
    .await
    {
        Ok(known) => known,
        Err(_) => return unavailable(),
    };
    if known_capabilities != capabilities {
        return invalid_input();
    }

    let password_hash = match hash_password(&input.password) {
        Ok(hash) => hash,
        Err(_) => return unavailable(),
    };
    let user_id = Uuid::now_v7();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let inserted = sqlx::query(
        r#"
        INSERT INTO staff_users (id, email, display_name, password_hash, role)
        VALUES ($1, $2, $3, $4, 'staff')
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(display_name)
    .bind(password_hash)
    .execute(&mut *transaction)
    .await;
    if let Err(error) = inserted {
        return if error
            .as_database_error()
            .and_then(|database| database.code())
            .as_deref()
            == Some("23505")
        {
            conflict()
        } else {
            unavailable()
        };
    }

    for capability in &capabilities {
        if sqlx::query(
            r#"
            INSERT INTO staff_capabilities (staff_user_id, capability_name)
            VALUES ($1, $2)
            "#,
        )
        .bind(user_id)
        .bind(capability)
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            return unavailable();
        }
    }
    if audit(&mut transaction, actor.id, "staff.create", user_id, None)
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }

    (
        StatusCode::CREATED,
        Json(StaffRecord {
            id: user_id,
            email,
            display_name: display_name.to_owned(),
            role: "staff".into(),
            capabilities,
            disabled: false,
        }),
    )
        .into_response()
}

#[utoipa::path(
    post,
    path = "/api/admin/staff/{staff_id}/disable",
    params(("staff_id" = Uuid, Path, description = "Staff user ID")),
    tag = "staff",
    request_body = DisableStaffRequest,
    responses(
        (status = 204, description = "Staff member disabled and sessions revoked"),
        (status = 401, description = "Authentication required", body = ErrorBody),
        (status = 403, description = "Capability required", body = ErrorBody),
        (status = 404, description = "Staff member not found", body = ErrorBody),
        (status = 409, description = "Safety rule prevents disabling", body = ErrorBody),
        (status = 422, description = "A reason is required", body = ErrorBody)
    )
)]
pub async fn disable(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(staff_id): Path<Uuid>,
    Json(input): Json<DisableStaffRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, STAFF_MANAGE) {
        return response.into_response();
    }
    if actor.id == staff_id {
        return safety_conflict("You cannot disable your own staff account.");
    }
    let reason = input.reason.trim();
    if reason.is_empty() || reason.len() > 500 {
        return invalid_reason();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let target_role = match sqlx::query_scalar::<_, String>(
        "SELECT role FROM staff_users WHERE id = $1 AND disabled_at IS NULL",
    )
    .bind(staff_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(role)) => role,
        Ok(None) => return not_found(),
        Err(_) => return unavailable(),
    };
    if target_role == "owner" {
        let active_owners: i64 = match sqlx::query_scalar(
            "SELECT count(*) FROM staff_users WHERE role = 'owner' AND disabled_at IS NULL",
        )
        .fetch_one(&pool)
        .await
        {
            Ok(count) => count,
            Err(_) => return unavailable(),
        };
        if active_owners <= 1 {
            return safety_conflict("The last active owner cannot be disabled.");
        }
    }

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if sqlx::query("UPDATE staff_users SET disabled_at = now(), updated_at = now() WHERE id = $1")
        .bind(staff_id)
        .execute(&mut *transaction)
        .await
        .is_err()
        || sqlx::query(
            "UPDATE staff_sessions SET revoked_at = now() WHERE staff_user_id = $1 AND revoked_at IS NULL",
        )
        .bind(staff_id)
        .execute(&mut *transaction)
        .await
        .is_err()
        || audit(
            &mut transaction,
            actor.id,
            "staff.disable",
            staff_id,
            Some(reason),
        )
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }

    StatusCode::NO_CONTENT.into_response()
}

async fn audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: Uuid,
    action: &str,
    entity_id: Uuid,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (
            actor_staff_user_id, action, entity_type, entity_id, reason
        )
        VALUES ($1, $2, 'staff_user', $3, $4)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(entity_id.to_string())
    .bind(reason)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "database_unavailable",
        "Staff management is temporarily unavailable.",
    )
}

fn invalid_input() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_staff_details",
        "Provide a valid email, name, password, and capability list.",
    )
}

fn invalid_reason() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "reason_required",
        "A reason is required to disable a staff account.",
    )
}

fn conflict() -> Response {
    error(
        StatusCode::CONFLICT,
        "staff_email_exists",
        "A staff account with that email already exists.",
    )
}

fn safety_conflict(message: &'static str) -> Response {
    error(StatusCode::CONFLICT, "staff_safety_rule", message)
}

fn not_found() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "staff_not_found",
        "The staff account was not found.",
    )
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}
