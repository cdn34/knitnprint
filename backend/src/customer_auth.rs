use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use time::Duration;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{hash_password, verify_password},
    customers::{CustomerAddress, valid_email},
    email::{AccountEmailKind, log_delivery_failure},
    error::ErrorBody,
    login_rate_limit::{AccountLoginGuard, AuthScope, ClientIp, LoginLimitError},
};

pub const CUSTOMER_SESSION_COOKIE: &str = "knitprint_customer";
const CUSTOMER_SESSION_DAYS: i64 = 30;
const ACCOUNT_EMAIL_RESEND_MINUTES: i32 = 5;

#[derive(Deserialize, ToSchema)]
pub struct CustomerRegisterRequest {
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(default)]
    pub phone: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CustomerLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct AccountTokenRequest {
    pub token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAccountAddressRequest {
    pub address_type: String,
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
pub struct CustomerAccountProfile {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email_verified: bool,
    pub addresses: Vec<CustomerAddress>,
}

#[derive(Clone, FromRow)]
struct AccountIdentity {
    id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    phone: String,
    email_verified: bool,
}

#[derive(FromRow)]
struct AccountCredentials {
    id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    phone: String,
    email_verified: bool,
    password_hash: String,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedCustomer {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub email_verified: bool,
}

#[utoipa::path(
    post,
    path = "/api/account/register",
    tag = "customer account",
    request_body = CustomerRegisterRequest,
    responses(
        (status = 201, body = CustomerAccountProfile),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<CustomerRegisterRequest>,
) -> Response {
    if !valid_registration(&input) {
        return invalid_registration();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let password_hash = match hash_password(&input.password) {
        Ok(hash) => hash,
        Err(_) => return unavailable(),
    };
    let customer_id = Uuid::now_v7();
    let session_id = Uuid::now_v7();
    let token = new_session_token();
    let token_hash = hash_token(&token);
    let email_token_id = Uuid::now_v7();
    let email_token = new_session_token();
    let email_token_hash = hash_token(&email_token);
    let email = input.email.trim().to_ascii_lowercase();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if let Err(error) = sqlx::query(
        r#"
        INSERT INTO customers (id, customer_type, email, first_name, last_name, phone)
        VALUES ($1, 'registered', $2, $3, $4, $5)
        "#,
    )
    .bind(customer_id)
    .bind(&email)
    .bind(input.first_name.trim())
    .bind(input.last_name.trim())
    .bind(input.phone.trim())
    .execute(&mut *transaction)
    .await
    {
        return registration_write_error(error);
    }
    if sqlx::query("INSERT INTO customer_accounts (customer_id, password_hash) VALUES ($1, $2)")
        .bind(customer_id)
        .bind(password_hash)
        .execute(&mut *transaction)
        .await
        .is_err()
        || insert_session(&mut transaction, session_id, customer_id, &token_hash)
            .await
            .is_err()
        || insert_account_token(
            &mut transaction,
            email_token_id,
            customer_id,
            AccountEmailKind::Verification,
            &email_token_hash,
        )
        .await
        .is_err()
        || audit(
            &mut transaction,
            customer_id,
            "customer.register",
            "customer",
            customer_id,
        )
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    if let Err(error) = state
        .email
        .send_account_action(
            &email,
            input.first_name.trim(),
            AccountEmailKind::Verification,
            &email_token,
        )
        .await
    {
        log_delivery_failure(&error, AccountEmailKind::Verification);
    }
    no_store(
        (
            StatusCode::CREATED,
            jar.add(session_cookie(token, state.secure_cookies)),
            Json(CustomerAccountProfile {
                id: customer_id,
                email,
                first_name: input.first_name.trim().into(),
                last_name: input.last_name.trim().into(),
                phone: input.phone.trim().into(),
                email_verified: false,
                addresses: Vec::new(),
            }),
        )
            .into_response(),
    )
}

#[utoipa::path(
    post,
    path = "/api/account/login",
    tag = "customer account",
    request_body = CustomerLoginRequest,
    responses(
        (status = 200, body = CustomerAccountProfile),
        (status = 401, body = ErrorBody),
        (status = 429, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ClientIp(client_ip): ClientIp,
    jar: CookieJar,
    Json(input): Json<CustomerLoginRequest>,
) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    let email = input.email.trim().to_ascii_lowercase();
    let mut guard = match AccountLoginGuard::acquire(
        &pool,
        AuthScope::Customer,
        bounded_identifier(&email),
        client_ip,
    )
    .await
    {
        Ok(guard) => guard,
        Err(LoginLimitError::Limited(retry_after)) => return login_limited(retry_after),
        Err(LoginLimitError::Database(_)) => return unavailable(),
    };
    if !valid_email(&email) || !(12..=256).contains(&input.password.len()) {
        if guard.record_failure().await.is_err() {
            return unavailable();
        }
        return invalid_credentials();
    }
    let credentials = match sqlx::query_as::<_, AccountCredentials>(
        r#"
        SELECT customer.id, customer.email::text AS email, customer.first_name,
               customer.last_name, customer.phone,
               account.email_verified_at IS NOT NULL AS email_verified,
               account.password_hash
        FROM customer_accounts account
        JOIN customers customer ON customer.id = account.customer_id
        WHERE customer.email = $1
          AND customer.customer_type = 'registered'
          AND customer.anonymized_at IS NULL
          AND customer.retention_expires_at > now()
          AND account.disabled_at IS NULL
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
    let session_id = Uuid::now_v7();
    let token = new_session_token();
    let token_hash = hash_token(&token);
    if lock_active_account(guard.transaction(), credentials.id)
        .await
        .is_err()
        || insert_session(guard.transaction(), session_id, credentials.id, &token_hash)
            .await
            .is_err()
        || refresh_retention(guard.transaction(), credentials.id)
            .await
            .is_err()
        || audit(
            guard.transaction(),
            credentials.id,
            "customer.login",
            "customer_session",
            session_id,
        )
        .await
        .is_err()
        || guard.record_success().await.is_err()
    {
        return unavailable();
    }
    let addresses = match addresses_for(&pool, credentials.id).await {
        Ok(addresses) => addresses,
        Err(_) => return unavailable(),
    };
    no_store(
        (
            jar.add(session_cookie(token, state.secure_cookies)),
            Json(CustomerAccountProfile {
                id: credentials.id,
                email: credentials.email,
                first_name: credentials.first_name,
                last_name: credentials.last_name,
                phone: credentials.phone,
                email_verified: credentials.email_verified,
                addresses,
            }),
        )
            .into_response(),
    )
}

#[utoipa::path(
    post,
    path = "/api/account/logout",
    tag = "customer account",
    responses((status = 204, description = "Customer session revoked"))
)]
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> Response {
    let Some(cookie) = jar.get(CUSTOMER_SESSION_COOKIE) else {
        return no_store((jar.remove(removal_cookie()), StatusCode::NO_CONTENT).into_response());
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let token_hash = hash_token(cookie.value());
    let revoked = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        UPDATE customer_sessions SET revoked_at = now()
        WHERE token_hash = $1 AND revoked_at IS NULL
        RETURNING id, customer_id
        "#,
    )
    .bind(token_hash.as_slice())
    .fetch_optional(&pool)
    .await;
    let (session_id, customer_id) = match revoked {
        Ok(Some(revoked)) => revoked,
        Ok(None) => {
            return no_store(
                (jar.remove(removal_cookie()), StatusCode::NO_CONTENT).into_response(),
            );
        }
        Err(_) => return unavailable(),
    };
    let _ = audit_pool(
        &pool,
        customer_id,
        "customer.logout",
        "customer_session",
        session_id,
    )
    .await;
    no_store((jar.remove(removal_cookie()), StatusCode::NO_CONTENT).into_response())
}

#[utoipa::path(
    get,
    path = "/api/account/me",
    tag = "customer account",
    responses(
        (status = 200, body = CustomerAccountProfile),
        (status = 401, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn me(State(state): State<AppState>, customer: AuthenticatedCustomer) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    match profile_for(&pool, customer).await {
        Ok(profile) => no_store(Json(profile).into_response()),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/account/addresses",
    tag = "customer account",
    request_body = CreateAccountAddressRequest,
    responses(
        (status = 201, body = CustomerAddress),
        (status = 401, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn add_address(
    State(state): State<AppState>,
    customer: AuthenticatedCustomer,
    Json(input): Json<CreateAccountAddressRequest>,
) -> Response {
    if !valid_address(&input) {
        return invalid_address();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let address = CustomerAddress {
        id: Uuid::now_v7(),
        address_type: input.address_type.trim().to_ascii_lowercase(),
        recipient_name: input.recipient_name.trim().into(),
        line1: input.line1.trim().into(),
        line2: input.line2.trim().into(),
        city: input.city.trim().into(),
        region: input.region.trim().into(),
        postal_code: input.postal_code.trim().into(),
        country_code: input.country_code.trim().to_ascii_uppercase(),
        phone: input.phone.trim().into(),
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if lock_active_account(&mut transaction, customer.id)
        .await
        .is_err()
        || sqlx::query(
            r#"
        INSERT INTO customer_addresses (
            id, customer_id, address_type, recipient_name, line1, line2, city,
            region, postal_code, country_code, phone
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
        )
        .bind(address.id)
        .bind(customer.id)
        .bind(&address.address_type)
        .bind(&address.recipient_name)
        .bind(&address.line1)
        .bind(&address.line2)
        .bind(&address.city)
        .bind(&address.region)
        .bind(&address.postal_code)
        .bind(&address.country_code)
        .bind(&address.phone)
        .execute(&mut *transaction)
        .await
        .is_err()
        || refresh_retention(&mut transaction, customer.id)
            .await
            .is_err()
        || audit(
            &mut transaction,
            customer.id,
            "customer.address_create",
            "customer_address",
            address.id,
        )
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    no_store((StatusCode::CREATED, Json(address)).into_response())
}

#[utoipa::path(
    post,
    path = "/api/account/verification/request",
    tag = "customer account",
    responses(
        (status = 204, description = "Verification email requested or recently sent"),
        (status = 401, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn request_verification(
    State(state): State<AppState>,
    customer: AuthenticatedCustomer,
) -> Response {
    if customer.email_verified {
        return no_store(StatusCode::NO_CONTENT.into_response());
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let token = new_session_token();
    let token_hash = hash_token(&token);
    let token_id = Uuid::now_v7();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if lock_active_account(&mut transaction, customer.id)
        .await
        .is_err()
    {
        return unavailable();
    }
    match recently_issued_token(
        &mut transaction,
        customer.id,
        AccountEmailKind::Verification,
    )
    .await
    {
        Ok(true) => {
            return if transaction.commit().await.is_ok() {
                no_store(StatusCode::NO_CONTENT.into_response())
            } else {
                unavailable()
            };
        }
        Ok(false) => {}
        Err(_) => return unavailable(),
    }
    if replace_account_token(
        &mut transaction,
        token_id,
        customer.id,
        AccountEmailKind::Verification,
        &token_hash,
    )
    .await
    .is_err()
        || audit(
            &mut transaction,
            customer.id,
            "customer.email_verification_requested",
            "customer",
            customer.id,
        )
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    if let Err(error) = state
        .email
        .send_account_action(
            &customer.email,
            &customer.first_name,
            AccountEmailKind::Verification,
            &token,
        )
        .await
    {
        log_delivery_failure(&error, AccountEmailKind::Verification);
    }
    no_store(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/account/verification/confirm",
    tag = "customer account",
    request_body = AccountTokenRequest,
    responses(
        (status = 204, description = "Email verified"),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn confirm_verification(
    State(state): State<AppState>,
    Json(input): Json<AccountTokenRequest>,
) -> Response {
    let Some(token_hash) = valid_action_token(&input.token) else {
        return invalid_action_token();
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let customer_id = match active_token_customer(
        &mut transaction,
        &token_hash,
        AccountEmailKind::Verification,
    )
    .await
    {
        Ok(Some(customer_id)) => customer_id,
        Ok(None) => return invalid_action_token(),
        Err(_) => return unavailable(),
    };
    if sqlx::query(
        "UPDATE customer_accounts SET email_verified_at = COALESCE(email_verified_at, now()), updated_at = now() WHERE customer_id = $1",
    )
    .bind(customer_id)
    .execute(&mut *transaction)
    .await
    .is_err()
        || use_account_tokens(
            &mut transaction,
            customer_id,
            AccountEmailKind::Verification,
        )
        .await
        .is_err()
        || audit(
            &mut transaction,
            customer_id,
            "customer.email_verified",
            "customer",
            customer_id,
        )
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    no_store(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/account/password/forgot",
    tag = "customer account",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 204, description = "Reset email requested when the account exists"),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(input): Json<ForgotPasswordRequest>,
) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    let email = input.email.trim().to_ascii_lowercase();
    let token = new_session_token();
    let token_hash = hash_token(&token);
    let token_id = Uuid::now_v7();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let account = if valid_email(&email) {
        sqlx::query_as::<_, (Uuid, String)>(
            r#"
            SELECT customer.id, customer.first_name
            FROM customer_accounts account
            JOIN customers customer ON customer.id = account.customer_id
            WHERE customer.email = $1
              AND customer.customer_type = 'registered'
              AND customer.anonymized_at IS NULL
              AND customer.retention_expires_at > now()
              AND account.disabled_at IS NULL
            FOR UPDATE OF account, customer
            "#,
        )
        .bind(&email)
        .fetch_optional(&mut *transaction)
        .await
    } else {
        Ok(None)
    };
    let Some((customer_id, first_name)) = (match account {
        Ok(account) => account,
        Err(_) => return unavailable(),
    }) else {
        if transaction.commit().await.is_err() {
            return unavailable();
        }
        let _ = hash_password("password-recovery-timing-padding");
        return no_store(StatusCode::NO_CONTENT.into_response());
    };
    match recently_issued_token(
        &mut transaction,
        customer_id,
        AccountEmailKind::PasswordReset,
    )
    .await
    {
        Ok(true) => {
            return if transaction.commit().await.is_ok() {
                no_store(StatusCode::NO_CONTENT.into_response())
            } else {
                unavailable()
            };
        }
        Ok(false) => {}
        Err(_) => return unavailable(),
    }
    if replace_account_token(
        &mut transaction,
        token_id,
        customer_id,
        AccountEmailKind::PasswordReset,
        &token_hash,
    )
    .await
    .is_err()
        || audit_system(
            &mut transaction,
            "customer.password_reset_requested",
            "customer",
            customer_id,
        )
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    if let Err(error) = state
        .email
        .send_account_action(&email, &first_name, AccountEmailKind::PasswordReset, &token)
        .await
    {
        log_delivery_failure(&error, AccountEmailKind::PasswordReset);
    }
    no_store(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/account/password/reset",
    tag = "customer account",
    request_body = ResetPasswordRequest,
    responses(
        (status = 204, description = "Password changed and existing sessions revoked"),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(input): Json<ResetPasswordRequest>,
) -> Response {
    let Some(token_hash) = valid_action_token(&input.token) else {
        return invalid_action_token();
    };
    if !(12..=256).contains(&input.password.len()) {
        return invalid_reset_password();
    }
    let password_hash = match hash_password(&input.password) {
        Ok(hash) => hash,
        Err(_) => return unavailable(),
    };
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let customer_id = match active_token_customer(
        &mut transaction,
        &token_hash,
        AccountEmailKind::PasswordReset,
    )
    .await
    {
        Ok(Some(customer_id)) => customer_id,
        Ok(None) => return invalid_action_token(),
        Err(_) => return unavailable(),
    };
    if sqlx::query(
        "UPDATE customer_accounts SET password_hash = $2, email_verified_at = COALESCE(email_verified_at, now()), updated_at = now() WHERE customer_id = $1",
    )
    .bind(customer_id)
    .bind(password_hash)
    .execute(&mut *transaction)
    .await
    .is_err()
        || use_account_tokens(
            &mut transaction,
            customer_id,
            AccountEmailKind::PasswordReset,
        )
        .await
        .is_err()
        || use_account_tokens(
            &mut transaction,
            customer_id,
            AccountEmailKind::Verification,
        )
        .await
        .is_err()
        || sqlx::query(
            "UPDATE customer_sessions SET revoked_at = COALESCE(revoked_at, now()) WHERE customer_id = $1",
        )
        .bind(customer_id)
        .execute(&mut *transaction)
        .await
        .is_err()
        || refresh_retention(&mut transaction, customer_id)
            .await
            .is_err()
        || audit_system(
            &mut transaction,
            "customer.password_reset",
            "customer",
            customer_id,
        )
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    no_store((jar.remove(removal_cookie()), StatusCode::NO_CONTENT).into_response())
}

impl FromRequestParts<AppState> for AuthenticatedCustomer {
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
        let Some(cookie) = jar.get(CUSTOMER_SESSION_COOKIE) else {
            return Err(authentication_required());
        };
        let token_hash = hash_token(cookie.value());
        let mut transaction = pool.begin().await.map_err(|_| unavailable())?;
        let identity = sqlx::query_as::<_, AccountIdentity>(
            r#"
            SELECT customer.id, customer.email::text AS email, customer.first_name,
                   customer.last_name, customer.phone,
                   account.email_verified_at IS NOT NULL AS email_verified
            FROM customer_sessions session
            JOIN customer_accounts account ON account.customer_id = session.customer_id
            JOIN customers customer ON customer.id = account.customer_id
            WHERE session.token_hash = $1
              AND session.revoked_at IS NULL
              AND session.expires_at > now()
              AND account.disabled_at IS NULL
              AND customer.customer_type = 'registered'
              AND customer.anonymized_at IS NULL
              AND customer.retention_expires_at > now()
            FOR UPDATE OF session, account, customer
            "#,
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(authentication_required)?;
        if sqlx::query("UPDATE customer_sessions SET last_seen_at = now() WHERE token_hash = $1")
            .bind(token_hash.as_slice())
            .execute(&mut *transaction)
            .await
            .is_err()
            || refresh_retention(&mut transaction, identity.id)
                .await
                .is_err()
            || transaction.commit().await.is_err()
        {
            return Err(unavailable());
        }
        Ok(Self {
            id: identity.id,
            email: identity.email,
            first_name: identity.first_name,
            last_name: identity.last_name,
            phone: identity.phone,
            email_verified: identity.email_verified,
        })
    }
}

async fn profile_for(
    pool: &PgPool,
    customer: AuthenticatedCustomer,
) -> Result<CustomerAccountProfile, sqlx::Error> {
    Ok(CustomerAccountProfile {
        addresses: addresses_for(pool, customer.id).await?,
        id: customer.id,
        email: customer.email,
        first_name: customer.first_name,
        last_name: customer.last_name,
        phone: customer.phone,
        email_verified: customer.email_verified,
    })
}

async fn insert_account_token(
    transaction: &mut Transaction<'_, Postgres>,
    token_id: Uuid,
    customer_id: Uuid,
    kind: AccountEmailKind,
    token_hash: &[u8; 32],
) -> Result<(), sqlx::Error> {
    let expires_at = match kind {
        AccountEmailKind::Verification => "24 hours",
        AccountEmailKind::PasswordReset => "1 hour",
    };
    sqlx::query(
        r#"
        INSERT INTO customer_account_tokens (
            id, customer_id, token_kind, token_hash, expires_at
        )
        VALUES ($1, $2, $3, $4, now() + $5::interval)
        "#,
    )
    .bind(token_id)
    .bind(customer_id)
    .bind(kind.as_str())
    .bind(token_hash.as_slice())
    .bind(expires_at)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn replace_account_token(
    transaction: &mut Transaction<'_, Postgres>,
    token_id: Uuid,
    customer_id: Uuid,
    kind: AccountEmailKind,
    token_hash: &[u8; 32],
) -> Result<(), sqlx::Error> {
    use_account_tokens(transaction, customer_id, kind).await?;
    insert_account_token(transaction, token_id, customer_id, kind, token_hash).await
}

async fn use_account_tokens(
    transaction: &mut Transaction<'_, Postgres>,
    customer_id: Uuid,
    kind: AccountEmailKind,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE customer_account_tokens SET used_at = now() WHERE customer_id = $1 AND token_kind = $2 AND used_at IS NULL",
    )
    .bind(customer_id)
    .bind(kind.as_str())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn recently_issued_token(
    transaction: &mut Transaction<'_, Postgres>,
    customer_id: Uuid,
    kind: AccountEmailKind,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM customer_account_tokens
            WHERE customer_id = $1
              AND token_kind = $2
              AND used_at IS NULL
              AND expires_at > now()
              AND created_at > now() - make_interval(mins => $3)
        )
        "#,
    )
    .bind(customer_id)
    .bind(kind.as_str())
    .bind(ACCOUNT_EMAIL_RESEND_MINUTES)
    .fetch_one(&mut **transaction)
    .await
}

async fn active_token_customer(
    transaction: &mut Transaction<'_, Postgres>,
    token_hash: &[u8; 32],
    kind: AccountEmailKind,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT token.customer_id
        FROM customer_account_tokens token
        JOIN customer_accounts account ON account.customer_id = token.customer_id
        JOIN customers customer ON customer.id = token.customer_id
        WHERE token.token_hash = $1
          AND token.token_kind = $2
          AND token.used_at IS NULL
          AND token.expires_at > now()
          AND account.disabled_at IS NULL
          AND customer.anonymized_at IS NULL
          AND customer.retention_expires_at > now()
        FOR UPDATE OF token, account, customer
        "#,
    )
    .bind(token_hash.as_slice())
    .bind(kind.as_str())
    .fetch_optional(&mut **transaction)
    .await
}

async fn addresses_for(
    pool: &PgPool,
    customer_id: Uuid,
) -> Result<Vec<CustomerAddress>, sqlx::Error> {
    sqlx::query_as::<_, CustomerAddress>(
        r#"
        SELECT id, address_type, recipient_name, line1, line2, city, region,
               postal_code, country_code::text AS country_code, phone
        FROM customer_addresses WHERE customer_id = $1 ORDER BY created_at, id
        "#,
    )
    .bind(customer_id)
    .fetch_all(pool)
    .await
}

async fn insert_session(
    transaction: &mut Transaction<'_, Postgres>,
    session_id: Uuid,
    customer_id: Uuid,
    token_hash: &[u8; 32],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO customer_sessions (id, customer_id, token_hash, expires_at)
        VALUES ($1, $2, $3, now() + interval '30 days')
        "#,
    )
    .bind(session_id)
    .bind(customer_id)
    .bind(token_hash.as_slice())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn refresh_retention(
    transaction: &mut Transaction<'_, Postgres>,
    customer_id: Uuid,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE customers SET retention_expires_at = now() + interval '24 months', updated_at = now()
        WHERE id = $1 AND customer_type = 'registered' AND anonymized_at IS NULL
        "#,
    )
    .bind(customer_id)
    .execute(&mut **transaction)
    .await?;
    if result.rows_affected() == 1 {
        Ok(())
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

async fn lock_active_account(
    transaction: &mut Transaction<'_, Postgres>,
    customer_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i32>(
        r#"
        SELECT 1
        FROM customer_accounts account
        JOIN customers customer ON customer.id = account.customer_id
        WHERE account.customer_id = $1
          AND account.disabled_at IS NULL
          AND customer.customer_type = 'registered'
          AND customer.anonymized_at IS NULL
          AND customer.retention_expires_at > now()
        FOR UPDATE OF account, customer
        "#,
    )
    .bind(customer_id)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(())
}

async fn audit(
    transaction: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_customer_id, action, entity_type, entity_id)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn audit_pool(
    pool: &PgPool,
    actor: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_customer_id, action, entity_type, entity_id)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

async fn audit_system(
    transaction: &mut Transaction<'_, Postgres>,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO audit_log (action, entity_type, entity_id) VALUES ($1, $2, $3)")
        .bind(action)
        .bind(entity_type)
        .bind(entity_id.to_string())
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

fn valid_registration(input: &CustomerRegisterRequest) -> bool {
    valid_email(&input.email)
        && (12..=256).contains(&input.password.len())
        && valid_required(&input.first_name, 100)
        && valid_required(&input.last_name, 100)
        && input.phone.trim().len() <= 40
}

fn valid_address(input: &CreateAccountAddressRequest) -> bool {
    matches!(input.address_type.trim(), "delivery" | "billing")
        && valid_required(&input.recipient_name, 200)
        && valid_required(&input.line1, 200)
        && input.line2.trim().len() <= 200
        && valid_required(&input.city, 120)
        && input.region.trim().len() <= 120
        && valid_required(&input.postal_code, 32)
        && input.country_code.trim().len() == 2
        && input
            .country_code
            .trim()
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        && input.phone.trim().len() <= 40
}

fn valid_required(value: &str, maximum: usize) -> bool {
    let value = value.trim();
    !value.is_empty() && value.len() <= maximum
}

fn registration_write_error(error: sqlx::Error) -> Response {
    if error
        .as_database_error()
        .and_then(|database| database.code())
        .as_deref()
        == Some("23505")
    {
        error_response(
            StatusCode::CONFLICT,
            "customer_email_exists",
            "A customer account with that email already exists.",
        )
    } else {
        unavailable()
    }
}

fn invalid_registration() -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_customer_account",
        "Provide a valid email, name, and password of at least 12 characters.",
    )
}

fn invalid_address() -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_customer_address",
        "Provide valid billing or delivery address details.",
    )
}

fn invalid_credentials() -> Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        "invalid_customer_credentials",
        "The email or password is incorrect.",
    )
}

fn invalid_action_token() -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_account_token",
        "This account link is invalid or has expired.",
    )
}

fn invalid_reset_password() -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_reset_password",
        "Choose a password of at least 12 characters.",
    )
}

fn authentication_required() -> Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        "customer_authentication_required",
        "Sign in to access this customer account.",
    )
}

fn login_limited(retry_after: u64) -> Response {
    let mut response = error_response(
        StatusCode::TOO_MANY_REQUESTS,
        "customer_login_rate_limited",
        "Too many sign-in attempts. Try again later.",
    );
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

fn unavailable() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "database_unavailable",
        "Customer accounts are temporarily unavailable.",
    )
}

fn error_response(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}

fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    response
}

fn new_session_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub(crate) fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

fn valid_action_token(token: &str) -> Option<[u8; 32]> {
    let token = token.trim();
    if token.len() == 64 && token.chars().all(|character| character.is_ascii_hexdigit()) {
        Some(hash_token(token))
    } else {
        None
    }
}

fn session_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((CUSTOMER_SESSION_COOKIE, token))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/api")
        .max_age(Duration::days(CUSTOMER_SESSION_DAYS))
        .build()
}

fn removal_cookie() -> Cookie<'static> {
    Cookie::build(CUSTOMER_SESSION_COOKIE)
        .path("/api")
        .max_age(Duration::ZERO)
        .build()
}

#[cfg(test)]
mod tests {
    use super::{
        CreateAccountAddressRequest, CustomerRegisterRequest, valid_address, valid_registration,
    };

    #[test]
    fn validates_registration_and_owned_addresses() {
        let registration = CustomerRegisterRequest {
            email: "customer@example.com".into(),
            password: "customer-passphrase".into(),
            first_name: "Marta".into(),
            last_name: "Silva".into(),
            phone: "".into(),
        };
        assert!(valid_registration(&registration));
        let address = CreateAccountAddressRequest {
            address_type: "delivery".into(),
            recipient_name: "Marta Silva".into(),
            line1: "24 Rua das Malhas".into(),
            line2: "".into(),
            city: "Porto".into(),
            region: "Porto".into(),
            postal_code: "4000-123".into(),
            country_code: "PT".into(),
            phone: "".into(),
        };
        assert!(valid_address(&address));
    }
}
