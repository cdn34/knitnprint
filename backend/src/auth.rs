use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use time::Duration;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    error::ErrorBody,
    login_rate_limit::{AccountLoginGuard, AuthScope, ClientIp, LoginLimitError},
};

pub const SESSION_COOKIE: &str = "knitprint_admin";
const SESSION_HOURS: i64 = 12;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct StaffProfile {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedStaff {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub capabilities: Vec<String>,
}

#[derive(FromRow)]
struct StaffCredentials {
    id: Uuid,
    email: String,
    display_name: String,
    role: String,
    password_hash: String,
}

#[derive(FromRow)]
struct SessionProfile {
    id: Uuid,
    email: String,
    display_name: String,
    role: String,
}

#[utoipa::path(
    post,
    path = "/api/admin/auth/login",
    tag = "staff auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated staff profile", body = StaffProfile),
        (status = 401, description = "Invalid credentials", body = ErrorBody),
        (status = 429, description = "Too many login attempts", body = ErrorBody),
        (status = 503, description = "Database unavailable", body = ErrorBody)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    jar: CookieJar,
    Json(input): Json<LoginRequest>,
) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    let email = input.email.trim().to_lowercase();
    let mut guard = match AccountLoginGuard::acquire(
        &pool,
        AuthScope::Staff,
        bounded_identifier(&email),
        client_ip,
    )
    .await
    {
        Ok(guard) => guard,
        Err(LoginLimitError::Limited(retry_after)) => return login_limited(retry_after),
        Err(LoginLimitError::Database(_)) => return unavailable(),
    };
    if email.is_empty()
        || email.len() > 254
        || input.password.is_empty()
        || input.password.len() > 256
    {
        if guard.record_failure().await.is_err() {
            return unavailable();
        }
        return invalid_credentials();
    }
    let credentials = match sqlx::query_as::<_, StaffCredentials>(
        r#"
        SELECT id, email::text AS email, display_name, role, password_hash
        FROM staff_users
        WHERE email = $1 AND disabled_at IS NULL
        "#,
    )
    .bind(&email)
    .fetch_optional(&mut **guard.transaction())
    .await
    {
        Ok(Some(credentials)) => credentials,
        Ok(None) => {
            let _ = hash_password(&input.password);
            if guard.record_failure().await.is_err() {
                return unavailable();
            }
            return invalid_credentials();
        }
        Err(_) => return unavailable(),
    };

    if !verify_password(&input.password, &credentials.password_hash) {
        if guard.record_failure().await.is_err() {
            return unavailable();
        }
        return invalid_credentials();
    }

    let token = new_session_token();
    let token_hash = hash_token(&token);
    let session_id = Uuid::now_v7();
    if sqlx::query(
        r#"
        INSERT INTO staff_sessions (id, staff_user_id, token_hash, expires_at)
        VALUES ($1, $2, $3, now() + interval '12 hours')
        "#,
    )
    .bind(session_id)
    .bind(credentials.id)
    .bind(token_hash.as_slice())
    .execute(&mut **guard.transaction())
    .await
    .is_err()
    {
        return unavailable();
    }

    if insert_audit(
        guard.transaction(),
        credentials.id,
        "staff.login",
        "staff_session",
        Some(session_id.to_string()),
    )
    .await
    .is_err()
        || guard.record_success().await.is_err()
    {
        return unavailable();
    }

    let capabilities = match capabilities_for(&pool, credentials.id, &credentials.role).await {
        Ok(capabilities) => capabilities,
        Err(_) => return unavailable(),
    };
    let cookie = session_cookie(token, state.secure_cookies);
    (
        jar.add(cookie),
        Json(StaffProfile {
            id: credentials.id,
            email: credentials.email,
            display_name: credentials.display_name,
            role: credentials.role,
            capabilities,
        }),
    )
        .into_response()
}

#[utoipa::path(
    post,
    path = "/api/admin/auth/logout",
    tag = "staff auth",
    responses((status = 204, description = "Session revoked"))
)]
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Response {
    if let (Some(pool), Some(cookie)) = (state.database, jar.get(SESSION_COOKIE)) {
        let token_hash = hash_token(cookie.value());
        if let Ok(mut transaction) = pool.begin().await
            && let Ok(Some((session_id, staff_user_id))) = sqlx::query_as::<_, (Uuid, Uuid)>(
                r#"
                UPDATE staff_sessions
                SET revoked_at = now()
                WHERE token_hash = $1 AND revoked_at IS NULL
                RETURNING id, staff_user_id
                "#,
            )
            .bind(token_hash.as_slice())
            .fetch_optional(&mut *transaction)
            .await
            && insert_audit(
                &mut transaction,
                staff_user_id,
                "staff.logout",
                "staff_session",
                Some(session_id.to_string()),
            )
            .await
            .is_ok()
        {
            let _ = transaction.commit().await;
        }
    }

    (jar.remove(removal_cookie()), StatusCode::NO_CONTENT).into_response()
}

#[utoipa::path(
    get,
    path = "/api/admin/auth/me",
    tag = "staff auth",
    responses(
        (status = 200, description = "Current staff profile", body = StaffProfile),
        (status = 401, description = "No active session", body = ErrorBody)
    )
)]
pub async fn me(staff: AuthenticatedStaff) -> Json<StaffProfile> {
    Json(staff.into_profile())
}

impl AuthenticatedStaff {
    pub fn has_capability(&self, capability: &str) -> bool {
        self.role == "owner"
            || self
                .capabilities
                .iter()
                .any(|granted| granted == capability)
    }

    pub fn into_profile(self) -> StaffProfile {
        StaffProfile {
            id: self.id,
            email: self.email,
            display_name: self.display_name,
            role: self.role,
            capabilities: self.capabilities,
        }
    }
}

impl FromRequestParts<AppState> for AuthenticatedStaff {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(pool) = state.database.as_ref() else {
            return Err(unavailable());
        };
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| authentication_required())?;
        let Some(cookie) = jar.get(SESSION_COOKIE) else {
            return Err(authentication_required());
        };
        let token_hash = hash_token(cookie.value());
        let profile = sqlx::query_as::<_, SessionProfile>(
            r#"
        SELECT u.id, u.email::text AS email, u.display_name, u.role
        FROM staff_sessions s
        JOIN staff_users u ON u.id = s.staff_user_id
        WHERE s.token_hash = $1
          AND s.revoked_at IS NULL
          AND s.expires_at > now()
          AND u.disabled_at IS NULL
        "#,
        )
        .bind(token_hash.as_slice())
        .fetch_optional(pool)
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(authentication_required)?;
        let capabilities = capabilities_for(pool, profile.id, &profile.role)
            .await
            .map_err(|_| unavailable())?;

        Ok(Self {
            id: profile.id,
            email: profile.email,
            display_name: profile.display_name,
            role: profile.role,
            capabilities,
        })
    }
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

pub(crate) fn verify_password(password: &str, encoded: &str) -> bool {
    let Ok(hash) = PasswordHash::new(encoded) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok()
}

fn new_session_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

fn session_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .path("/api/admin")
        .max_age(Duration::hours(SESSION_HOURS))
        .build()
}

fn removal_cookie() -> Cookie<'static> {
    Cookie::build(SESSION_COOKIE)
        .path("/api/admin")
        .max_age(Duration::ZERO)
        .build()
}

async fn capabilities_for(
    pool: &PgPool,
    user_id: Uuid,
    role: &str,
) -> Result<Vec<String>, sqlx::Error> {
    if role == "owner" {
        return sqlx::query_scalar("SELECT name FROM capabilities ORDER BY name")
            .fetch_all(pool)
            .await;
    }
    sqlx::query_scalar(
        r#"
        SELECT capability_name
        FROM staff_capabilities
        WHERE staff_user_id = $1
        ORDER BY capability_name
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn insert_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Option<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub fn require_capability(
    staff: &AuthenticatedStaff,
    capability: &'static str,
) -> Result<(), (StatusCode, Json<ErrorBody>)> {
    if staff.has_capability(capability) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(ErrorBody::new(
                "capability_required",
                "Your staff account cannot perform this operation.",
            )),
        ))
    }
}

fn invalid_credentials() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorBody::new(
            "invalid_credentials",
            "The email or password is incorrect.",
        )),
    )
        .into_response()
}

fn login_limited(retry_after: u64) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(ErrorBody::new(
            "login_rate_limited",
            "Too many sign-in attempts. Try again later.",
        )),
    )
        .into_response();
    if let Ok(value) = header::HeaderValue::from_str(&retry_after.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

fn bounded_identifier(identifier: &str) -> &str {
    if identifier.len() <= 320 {
        identifier
    } else {
        "__invalid__"
    }
}

fn authentication_required() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorBody::new(
            "authentication_required",
            "A valid staff session is required.",
        )),
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody::new(
            "database_unavailable",
            "Authentication is temporarily unavailable.",
        )),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{AuthenticatedStaff, hash_password, hash_token, verify_password};
    use uuid::Uuid;

    #[test]
    fn passwords_are_hashed_and_verified() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert_ne!(hash, "correct horse battery staple");
        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("wrong password", &hash));
    }

    #[test]
    fn session_tokens_are_not_stored_verbatim() {
        let digest = hash_token("opaque-session-token");
        assert_eq!(digest.len(), 32);
        assert_ne!(digest.as_slice(), b"opaque-session-token");
    }

    #[test]
    fn owners_have_every_capability() {
        let owner = AuthenticatedStaff {
            id: Uuid::nil(),
            email: "owner@example.com".into(),
            display_name: "Owner".into(),
            role: "owner".into(),
            capabilities: vec![],
        };
        assert!(owner.has_capability("staff.manage"));
        assert!(owner.has_capability("future.capability"));
    }

    #[test]
    fn staff_only_have_explicit_capabilities() {
        let staff = AuthenticatedStaff {
            id: Uuid::nil(),
            email: "staff@example.com".into(),
            display_name: "Staff".into(),
            role: "staff".into(),
            capabilities: vec!["catalog.read".into()],
        };
        assert!(staff.has_capability("catalog.read"));
        assert!(!staff.has_capability("staff.manage"));
    }
}
