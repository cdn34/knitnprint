//! Google credentials establish a local opaque session, never an API bearer token.
use crate::{
    AppState,
    auth::{hash_password, verify_password},
    customer_auth::{self as account, AuthenticatedCustomer},
    login_rate_limit::{ClientIp, consume_account_action},
};
use account::{error_response, hash_token, new_session_token, no_store, session_cookie};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

const BROWSER: &str = "knitnprint_google_browser";
#[derive(Serialize, ToSchema)]
pub struct AuthMethods {
    pub password: bool,
    pub google_email: Option<String>,
}
pub async fn methods(pool: &PgPool, id: Uuid) -> Result<AuthMethods, sqlx::Error> {
    let (password, google_email) = sqlx::query_as::<_, (bool, Option<String>)>("SELECT a.password_hash IS NOT NULL, g.email::text FROM customer_accounts a LEFT JOIN customer_google_identities g ON g.customer_id=a.customer_id WHERE a.customer_id=$1").bind(id).fetch_one(pool).await?;
    Ok(AuthMethods {
        password,
        google_email,
    })
}
#[derive(Serialize, ToSchema)]
pub struct GoogleConfig {
    pub client_id: Option<String>,
}
#[utoipa::path(get, path="/api/account/google/config", responses((status=200, body=GoogleConfig)))]
pub async fn config(State(state): State<AppState>) -> Response {
    no_store(
        Json(GoogleConfig {
            client_id: state.google.client_id.clone(),
        })
        .into_response(),
    )
}
#[derive(Deserialize, ToSchema)]
pub struct ChallengeRequest {
    pub purpose: String,
}
#[derive(Serialize, ToSchema)]
pub struct ChallengeResponse {
    pub id: Uuid,
    pub nonce: String,
}
#[utoipa::path(post, path="/api/account/google/challenge", request_body=ChallengeRequest, responses((status=200, body=ChallengeResponse)))]
pub async fn challenge(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    jar: CookieJar,
    Json(input): Json<ChallengeRequest>,
) -> Response {
    if state.google.client_id.is_none() {
        return failure("google_unavailable");
    }
    if !["sign_in", "link", "reauthenticate"].contains(&input.purpose.as_str()) {
        return failure("invalid_google_purpose");
    }
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    if crate::login_rate_limit::consume_request_volume(
        pool,
        crate::login_rate_limit::AuthScope::Customer,
        ip,
    )
    .await
    .is_err()
    {
        return limited();
    }
    let session = jar
        .get(account::CUSTOMER_SESSION_COOKIE)
        .map(|c| hash_token(c.value()));
    let customer = if input.purpose != "sign_in" {
        match session_customer(pool, session.as_ref()).await {
            Ok(id) => Some(id),
            Err(r) => return r,
        }
    } else {
        None
    };
    let browser = jar
        .get(BROWSER)
        .map(|c| c.value().to_owned())
        .unwrap_or_else(new_session_token);
    let nonce = new_session_token();
    let id = Uuid::now_v7();
    if sqlx::query("INSERT INTO customer_google_challenges(id,browser_hash,nonce_hash,purpose,customer_id,session_hash) VALUES($1,$2,$3,$4,$5,$6)").bind(id).bind(hash_token(&browser).as_slice()).bind(hash_token(&nonce).as_slice()).bind(&input.purpose).bind(customer).bind(if customer.is_some() { session.map(|h|h.to_vec()) } else {None}).execute(pool).await.is_err() { return unavailable(); }
    let cookie = Cookie::build((BROWSER, browser))
        .path("/api/account")
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(state.secure_cookies)
        .max_age(time::Duration::minutes(15))
        .build();
    no_store((jar.add(cookie), Json(ChallengeResponse { id, nonce })).into_response())
}
struct CachedKeys {
    expires_at: std::time::Instant,
    refreshed_at: std::time::Instant,
    keys: jsonwebtoken::jwk::JwkSet,
}
#[derive(Clone, Default)]
pub struct GoogleService {
    pub client_id: Option<String>,
    keys: std::sync::Arc<tokio::sync::Mutex<Option<CachedKeys>>>,
}
#[derive(Deserialize)]
struct Claims {
    sub: String,
    email: String,
    email_verified: bool,
    nonce: String,
    #[serde(default)]
    hd: Option<String>,
    #[serde(default)]
    given_name: String,
    #[serde(default)]
    family_name: String,
}
impl GoogleService {
    pub fn from_env() -> Result<Self, String> {
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .ok()
            .filter(|s| !s.is_empty());
        if client_id.as_ref().is_some_and(|s| {
            !s.ends_with(".apps.googleusercontent.com") || s.chars().any(char::is_whitespace)
        }) {
            return Err("Invalid GOOGLE_CLIENT_ID".into());
        }
        Ok(Self {
            client_id,
            ..Self::default()
        })
    }
    async fn verify(&self, token: &str) -> Result<Claims, ()> {
        let client_id = self.client_id.as_ref().ok_or(())?;
        if token.len() > 16384 {
            return Err(());
        }
        let header = jsonwebtoken::decode_header(token).map_err(|_| ())?;
        if header.alg != jsonwebtoken::Algorithm::RS256 {
            return Err(());
        }
        let kid = header.kid.ok_or(())?;
        let mut cache = self.keys.lock().await;
        if cache.as_ref().is_none_or(|cached| {
            cached.expires_at <= std::time::Instant::now()
                || (cached.keys.find(&kid).is_none()
                    && cached.refreshed_at.elapsed() >= std::time::Duration::from_secs(60))
        }) {
            let response = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .map_err(|_| ())?
                .get("https://www.googleapis.com/oauth2/v3/certs")
                .send()
                .await
                .map_err(|_| ())?
                .error_for_status()
                .map_err(|_| ())?;
            let age = response
                .headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| {
                    v.split(',')
                        .find_map(|v| v.trim().strip_prefix("max-age=")?.parse::<u64>().ok())
                })
                .unwrap_or(300)
                .min(86400);
            let keys = response.json().await.map_err(|_| ())?;
            *cache = Some(CachedKeys {
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(age),
                refreshed_at: std::time::Instant::now(),
                keys,
            });
        }
        let key = jsonwebtoken::DecodingKey::from_jwk(
            cache.as_ref().ok_or(())?.keys.find(&kid).ok_or(())?,
        )
        .map_err(|_| ())?;
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&[client_id]);
        validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);
        let claims = jsonwebtoken::decode::<Claims>(token, &key, &validation)
            .map_err(|_| ())?
            .claims;
        if !claims.email_verified
            || claims.sub.is_empty()
            || claims.sub.len() > 255
            || !crate::customers::valid_email(&claims.email)
        {
            return Err(());
        }
        Ok(claims)
    }
}
#[derive(Deserialize, ToSchema)]
pub struct GoogleRequest {
    pub challenge_id: Uuid,
    pub credential: String,
}
#[derive(Deserialize, ToSchema)]
pub struct GoogleConfirmRequest {
    pub challenge_id: Uuid,
    pub password: Option<String>,
    pub token: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}
#[derive(Serialize, ToSchema)]
pub struct GoogleResult {
    pub status: String,
    pub profile: Option<account::CustomerAccountProfile>,
    pub google_authenticated_until: Option<i64>,
}
#[derive(sqlx::FromRow)]
struct Pending {
    purpose: String,
    customer_id: Option<Uuid>,
    session_hash: Option<Vec<u8>>,
    nonce_hash: Vec<u8>,
    subject: Option<String>,
    email: Option<String>,
    first_name: String,
    last_name: String,
    trusted_email: bool,
    confirmed: bool,
    challenge_created_at: i64,
}
async fn pending(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    jar: &CookieJar,
) -> Result<Pending, Response> {
    let browser = jar
        .get(BROWSER)
        .ok_or_else(|| failure("google_challenge_expired"))?;
    sqlx::query_as::<_,Pending>("SELECT purpose,customer_id,session_hash,nonce_hash,subject,email::text,first_name,last_name,trusted_email,confirmed_at IS NOT NULL AS confirmed, floor(extract(epoch FROM created_at))::bigint AS challenge_created_at FROM customer_google_challenges WHERE id=$1 AND browser_hash=$2 AND expires_at>now() FOR UPDATE").bind(id).bind(hash_token(browser.value()).as_slice()).fetch_optional(&mut **tx).await.map_err(|_|unavailable())?.ok_or_else(||failure("google_challenge_expired"))
}
#[utoipa::path(post,path="/api/account/google",request_body=GoogleRequest,responses((status=200,body=GoogleResult)))]
pub async fn authenticate(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    jar: CookieJar,
    Json(input): Json<GoogleRequest>,
) -> Response {
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    if crate::login_rate_limit::consume_request_volume(
        pool,
        crate::login_rate_limit::AuthScope::Customer,
        ip,
    )
    .await
    .is_err()
    {
        return limited();
    }
    let claims = match state.google.verify(&input.credential).await {
        Ok(c) => c,
        Err(_) => return failure("invalid_google_credential"),
    };
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(_) => return unavailable(),
    };
    let p = match pending(&mut tx, input.challenge_id, &jar).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    if p.subject.is_some() || p.nonce_hash != hash_token(&claims.nonce) {
        return failure("invalid_google_nonce");
    }
    let email = claims.email.trim().to_ascii_lowercase();
    let trusted =
        email.ends_with("@gmail.com") || claims.hd.as_ref().is_some_and(|s| !s.is_empty());
    if sqlx::query("UPDATE customer_google_challenges SET subject=$2,email=$3,first_name=$4,last_name=$5,trusted_email=$6 WHERE id=$1").bind(input.challenge_id).bind(claims.sub).bind(email).bind(claims.given_name).bind(claims.family_name).bind(trusted).execute(&mut *tx).await.is_err() || tx.commit().await.is_err(){return unavailable()}
    complete(
        state,
        jar,
        GoogleConfirmRequest {
            challenge_id: input.challenge_id,
            password: None,
            token: None,
            first_name: None,
            last_name: None,
        },
    )
    .await
}
#[utoipa::path(post,path="/api/account/google/confirm",request_body=GoogleConfirmRequest,responses((status=200,body=GoogleResult)))]
pub async fn confirm(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    jar: CookieJar,
    Json(input): Json<GoogleConfirmRequest>,
) -> Response {
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    let Some(browser) = jar.get(BROWSER) else {
        return failure("google_challenge_expired");
    };
    let email = match sqlx::query_scalar::<_, Option<String>>("SELECT email::text FROM customer_google_challenges WHERE id=$1 AND browser_hash=$2 AND expires_at>now()")
        .bind(input.challenge_id).bind(hash_token(browser.value()).as_slice()).fetch_optional(pool).await {
        Ok(Some(Some(email))) => email,
        Ok(_) => return failure("google_challenge_expired"),
        Err(_) => return unavailable(),
    };
    if consume_account_action(pool, "google_confirm", &email, ip)
        .await
        .is_err()
    {
        return limited();
    }
    complete(state, jar, input).await
}
fn result(status: &str) -> Response {
    no_store(
        Json(GoogleResult {
            status: status.into(),
            profile: None,
            google_authenticated_until: None,
        })
        .into_response(),
    )
}
async fn complete(state: AppState, jar: CookieJar, input: GoogleConfirmRequest) -> Response {
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(_) => return unavailable(),
    };
    let mut p = match pending(&mut tx, input.challenge_id, &jar).await {
        Ok(p) => p,
        Err(r) => return r,
    };
    let (Some(subject), Some(email)) = (&p.subject, &p.email) else {
        return failure("invalid_google_challenge");
    };
    // Serialize signup/link attempts for both subject and email across backend instances.
    for key in [format!("google:{subject}"), format!("email:{email}")] {
        if sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(key)
            .execute(&mut *tx)
            .await
            .is_err()
        {
            return unavailable();
        }
    }
    let linked = match sqlx::query_as::<_, (Uuid, String)>(
        "SELECT customer_id,email::text FROM customer_google_identities WHERE subject=$1",
    )
    .bind(subject)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(v) => v,
        Err(_) => return unavailable(),
    };
    let existing=match sqlx::query_as::<_,(Uuid,Option<String>,bool,bool)>("SELECT c.id,a.password_hash,a.email_verified_at IS NOT NULL,a.google_auto_link_disabled FROM customers c JOIN customer_accounts a ON a.customer_id=c.id WHERE c.email=$1 AND c.customer_type='registered' AND c.anonymized_at IS NULL AND c.retention_expires_at>now() AND a.disabled_at IS NULL FOR UPDATE OF c,a").bind(email).fetch_optional(&mut *tx).await{Ok(v)=>v,Err(_)=>return unavailable()};
    if let Some((id, _)) = &linked
        && existing.as_ref().is_none_or(|e| e.0 != *id)
    {
        return failure("google_email_mismatch");
    }
    if p.purpose != "sign_in" {
        let hash = jar
            .get(account::CUSTOMER_SESSION_COOKIE)
            .map(|c| hash_token(c.value()));
        if hash.as_ref().map(|h| h.as_slice()) != p.session_hash.as_deref()
            || existing.as_ref().map(|e| e.0) != p.customer_id
        {
            return failure("google_account_mismatch");
        }
        if session_customer(pool, hash.as_ref()).await.ok() != p.customer_id {
            return failure("authentication_required");
        }
    }
    if p.purpose == "reauthenticate" {
        if linked.as_ref().map(|v| v.0) != p.customer_id {
            return failure("google_not_linked");
        }
        let until = match mark_google_authenticated(
            &mut tx,
            p.session_hash.as_deref().unwrap(),
            p.challenge_created_at,
        )
        .await
        {
            Ok(Some(until)) => until,
            Ok(None) => return failure("reauthentication_required"),
            Err(_) => return unavailable(),
        };
        if finish_challenge(&mut tx, input.challenge_id).await.is_err()
            || tx.commit().await.is_err()
        {
            return unavailable();
        }
        return no_store(
            Json(GoogleResult {
                status: "reauthenticated".into(),
                profile: None,
                google_authenticated_until: Some(until),
            })
            .into_response(),
        );
    }
    if p.purpose == "link"
        && let Err(r) =
            require_password_reauth_tx(&mut tx, &jar, p.customer_id.unwrap(), false).await
    {
        return r;
    }
    let needs_proof =
        linked.is_none() && (!p.trusted_email || existing.as_ref().is_some_and(|e| e.3));
    if let Some(token) = &input.token {
        let valid = sqlx::query_scalar::<_, bool>(
            "SELECT confirmation_hash=$2 FROM customer_google_challenges WHERE id=$1",
        )
        .bind(input.challenge_id)
        .bind(hash_token(token).as_slice())
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(false);
        if !valid {
            return failure("invalid_confirmation_token");
        }
        p.confirmed = true;
    }
    if let Some(password) = &input.password {
        if password.len() > 256
            || !existing
                .as_ref()
                .is_some_and(|e| e.2 && e.1.as_ref().is_some_and(|h| verify_password(password, h)))
        {
            return failure("invalid_credentials");
        }
        p.confirmed = true;
    }
    if p.purpose == "link" {
        p.confirmed = true;
    }
    if needs_proof && !p.confirmed {
        let token = new_session_token();
        let issued=sqlx::query("UPDATE customer_google_challenges SET confirmation_hash=$2,customer_id=$3 WHERE id=$1 AND confirmation_hash IS NULL").bind(input.challenge_id).bind(hash_token(&token).as_slice()).bind(existing.as_ref().map(|e|e.0)).execute(&mut *tx).await;
        let send = match issued {
            Ok(r) => r.rows_affected() == 1,
            Err(_) => return unavailable(),
        };
        if tx.commit().await.is_err() {
            return unavailable();
        }
        if send
            && state
                .email
                .send_google_confirmation(email, &token, input.challenge_id)
                .await
                .is_err()
        {
            return unavailable();
        }
        return result("confirmation_required");
    }
    if p.confirmed
        && sqlx::query("UPDATE customer_google_challenges SET confirmed_at=now() WHERE id=$1")
            .bind(input.challenge_id)
            .execute(&mut *tx)
            .await
            .is_err()
    {
        return unavailable();
    }
    let first = input.first_name.as_ref().unwrap_or(&p.first_name).trim();
    let last = input.last_name.as_ref().unwrap_or(&p.last_name).trim();
    let id = if let Some((id, _, _, _)) = &existing {
        *id
    } else {
        if first.is_empty() || last.is_empty() || first.len() > 100 || last.len() > 100 {
            if tx.commit().await.is_err() {
                return unavailable();
            }
            return result("profile_required");
        }
        // Refuse disabled/expired records rather than reactivating or overwriting them.
        let id = Uuid::now_v7();
        if sqlx::query("INSERT INTO customers(id,customer_type,email,first_name,last_name) VALUES($1,'registered',$2,$3,$4)").bind(id).bind(email).bind(first).bind(last).execute(&mut *tx).await.is_err() || sqlx::query("INSERT INTO customer_accounts(customer_id,email_verified_at) VALUES($1,now())").bind(id).execute(&mut *tx).await.is_err(){return failure("customer_email_unavailable")}
        id
    };
    let adding = linked.is_none();
    if adding {
        if existing.as_ref().is_some_and(|e| !e.2)
            && sqlx::query("UPDATE customer_accounts SET password_hash=NULL WHERE customer_id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .is_err()
        {
            return unavailable();
        }
        if sqlx::query(
            "INSERT INTO customer_google_identities(customer_id,subject,email) VALUES($1,$2,$3)",
        )
        .bind(id)
        .bind(subject)
        .bind(email)
        .execute(&mut *tx)
        .await
        .is_err()
        {
            return failure("google_already_linked");
        }
        if invalidate(&mut tx, id).await.is_err() {
            return unavailable();
        }
        if sqlx::query("UPDATE customer_accounts SET email_verified_at=COALESCE(email_verified_at,now()) WHERE customer_id=$1").bind(id).execute(&mut *tx).await.is_err(){return unavailable()}
    }
    if finish_challenge(&mut tx, input.challenge_id).await.is_err() {
        return unavailable();
    }
    let token = new_session_token();
    if account::insert_session(&mut tx, Uuid::now_v7(), id, &hash_token(&token))
        .await
        .is_err()
        || account::refresh_retention(&mut tx, id).await.is_err()
        || account::audit(
            &mut tx,
            id,
            if adding {
                "customer.google_link"
            } else {
                "customer.google_login"
            },
            "customer",
            id,
        )
        .await
        .is_err()
    {
        return unavailable();
    }
    // The Google credential already proves ownership. Preserve its original challenge time
    // so deferred email/profile completion cannot extend the five-minute security window.
    let google_authenticated_until =
        match mark_google_authenticated(&mut tx, &hash_token(&token), p.challenge_created_at).await
        {
            Ok(until) => until,
            Err(_) => return unavailable(),
        };
    if tx.commit().await.is_err() {
        return unavailable();
    }
    if adding {
        let _ = state
            .email
            .send_security_notice(email, "Google sign-in connected")
            .await;
    }
    let response = profile(pool, id).await;
    match response {
        Ok(profile) => no_store(
            (
                jar.add(session_cookie(token, state.secure_cookies)),
                Json(GoogleResult {
                    status: "authenticated".into(),
                    profile: Some(profile),
                    google_authenticated_until,
                }),
            )
                .into_response(),
        ),
        Err(_) => unavailable(),
    }
}
/// Return only the active caller session's current confirmation window.
pub(crate) async fn recent_google_authentication(
    pool: &PgPool,
    jar: &CookieJar,
    customer_id: Uuid,
) -> Result<Option<i64>, sqlx::Error> {
    let Some(cookie) = jar.get(account::CUSTOMER_SESSION_COOKIE) else {
        return Ok(None);
    };
    sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM least(reauthenticated_at+interval '5 minutes',expires_at)))::bigint FROM customer_sessions WHERE customer_id=$1 AND token_hash=$2 AND revoked_at IS NULL AND expires_at>now() AND reauthenticated_method='google' AND reauthenticated_at>now()-interval '5 minutes'"
    ).bind(customer_id).bind(hash_token(cookie.value()).as_slice()).fetch_optional(pool).await
}

async fn mark_google_authenticated(
    tx: &mut Transaction<'_, Postgres>,
    session_hash: &[u8],
    authenticated_at: i64,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar(
        "UPDATE customer_sessions SET reauthenticated_at=to_timestamp($2::bigint::double precision), reauthenticated_method='google' WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>now() AND to_timestamp($2::bigint::double precision)>now()-interval '5 minutes' RETURNING floor(extract(epoch FROM reauthenticated_at+interval '5 minutes'))::bigint"
    ).bind(session_hash).bind(authenticated_at).fetch_optional(&mut **tx).await
}

async fn finish_challenge(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM customer_google_challenges WHERE id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
async fn profile(pool: &PgPool, id: Uuid) -> Result<account::CustomerAccountProfile, sqlx::Error> {
    let (email, first_name, last_name, phone) =
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT email::text,first_name,last_name,phone FROM customers WHERE id=$1",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
    account::profile_for(
        pool,
        AuthenticatedCustomer {
            id,
            email,
            first_name,
            last_name,
            phone,
            email_verified: true,
        },
    )
    .await
}
async fn session_customer(pool: &PgPool, hash: Option<&[u8; 32]>) -> Result<Uuid, Response> {
    let hash = hash.ok_or_else(|| failure("authentication_required"))?;
    sqlx::query_scalar("SELECT c.id FROM customer_sessions s JOIN customer_accounts a ON a.customer_id=s.customer_id JOIN customers c ON c.id=a.customer_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>now() AND a.disabled_at IS NULL AND c.anonymized_at IS NULL AND c.retention_expires_at>now() AND a.email_verified_at IS NOT NULL").bind(hash.as_slice()).fetch_optional(pool).await.map_err(|_|unavailable())?.ok_or_else(||failure("authentication_required"))
}
#[derive(Deserialize, ToSchema)]
pub struct PasswordRequest {
    pub password: String,
}
#[utoipa::path(post,path="/api/account/reauthenticate",request_body=PasswordRequest,responses((status=204)))]
pub async fn reauthenticate(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    jar: CookieJar,
    customer: AuthenticatedCustomer,
    Json(input): Json<PasswordRequest>,
) -> Response {
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    if consume_account_action(pool, "reauthenticate", &customer.email, ip)
        .await
        .is_err()
    {
        return limited();
    }
    if input.password.len() > 256 {
        return failure("invalid_credentials");
    }
    let hash = sqlx::query_scalar::<_, Option<String>>(
        "SELECT password_hash FROM customer_accounts WHERE customer_id=$1",
    )
    .bind(customer.id)
    .fetch_one(pool)
    .await
    .ok()
    .flatten();
    if !customer.email_verified || !hash.is_some_and(|h| verify_password(&input.password, &h)) {
        return failure("invalid_credentials");
    }
    let Some(cookie) = jar.get(account::CUSTOMER_SESSION_COOKIE) else {
        return failure("authentication_required");
    };
    if sqlx::query("UPDATE customer_sessions SET reauthenticated_at=now(),reauthenticated_method='password' WHERE token_hash=$1 AND revoked_at IS NULL AND expires_at>now()").bind(hash_token(cookie.value()).as_slice()).execute(pool).await.is_err(){return unavailable()}
    no_store(StatusCode::NO_CONTENT.into_response())
}
pub(crate) async fn require_password_reauth(
    pool: &PgPool,
    jar: &CookieJar,
    id: Uuid,
) -> Result<(), Response> {
    let mut tx = pool.begin().await.map_err(|_| unavailable())?;
    require_password_reauth_tx(&mut tx, jar, id, true).await
}
async fn require_password_reauth_tx(
    tx: &mut Transaction<'_, Postgres>,
    jar: &CookieJar,
    id: Uuid,
    lock_email: bool,
) -> Result<(), Response> {
    if lock_email
        && sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM customer_google_identities WHERE customer_id=$1)",
        )
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .unwrap_or(true)
    {
        return Err(failure("google_email_locked"));
    }
    require_reauth(tx, jar, id, "password").await
}
async fn require_reauth(
    tx: &mut Transaction<'_, Postgres>,
    jar: &CookieJar,
    id: Uuid,
    method: &str,
) -> Result<(), Response> {
    let cookie = jar
        .get(account::CUSTOMER_SESSION_COOKIE)
        .ok_or_else(|| failure("authentication_required"))?;
    let valid=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM customer_sessions WHERE customer_id=$1 AND token_hash=$2 AND revoked_at IS NULL AND expires_at>now() AND reauthenticated_at>now()-interval '5 minutes' AND reauthenticated_method=$3)").bind(id).bind(hash_token(cookie.value()).as_slice()).bind(method).fetch_one(&mut **tx).await.map_err(|_|unavailable())?;
    if !valid {
        return Err(failure("reauthentication_required"));
    }
    Ok(())
}
async fn invalidate(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE customer_sessions SET revoked_at=COALESCE(revoked_at,now()) WHERE customer_id=$1",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    sqlx::query("DELETE FROM customer_account_tokens WHERE customer_id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM customer_google_challenges WHERE customer_id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
#[utoipa::path(put,path="/api/account/password",request_body=PasswordRequest,responses((status=200,body=account::CustomerAccountProfile)))]
pub async fn password(
    State(state): State<AppState>,
    jar: CookieJar,
    customer: AuthenticatedCustomer,
    Json(input): Json<PasswordRequest>,
) -> Response {
    mutate(state, jar, customer, Some(input.password), None).await
}
#[utoipa::path(delete,path="/api/account/auth-methods/{method}",params(("method"=String,Path)),responses((status=200,body=account::CustomerAccountProfile)))]
pub async fn unlink(
    State(state): State<AppState>,
    jar: CookieJar,
    customer: AuthenticatedCustomer,
    Path(method): Path<String>,
) -> Response {
    mutate(state, jar, customer, None, Some(method)).await
}
async fn mutate(
    state: AppState,
    jar: CookieJar,
    customer: AuthenticatedCustomer,
    password: Option<String>,
    remove: Option<String>,
) -> Response {
    if !customer.email_verified {
        return failure("email_verification_required");
    }
    let Some(pool) = state.database.as_ref() else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(_) => return unavailable(),
    };
    if account::lock_active_account(&mut tx, customer.id)
        .await
        .is_err()
    {
        return unavailable();
    }
    let has_password = sqlx::query_scalar::<_, bool>(
        "SELECT password_hash IS NOT NULL FROM customer_accounts WHERE customer_id=$1",
    )
    .bind(customer.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(false);
    let has_google = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM customer_google_identities WHERE customer_id=$1)",
    )
    .bind(customer.id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(false);
    let proof = if remove.as_deref() == Some("password") || password.is_some() && !has_password {
        "google"
    } else {
        "password"
    };
    if let Err(r) = require_reauth(&mut tx, &jar, customer.id, proof).await {
        return r;
    }
    let action = if let Some(password) = password {
        if !(12..=256).contains(&password.len()) {
            return failure("invalid_password");
        }
        let hash = match hash_password(&password) {
            Ok(h) => h,
            Err(_) => return unavailable(),
        };
        if sqlx::query(
            "UPDATE customer_accounts SET password_hash=$2,updated_at=now() WHERE customer_id=$1",
        )
        .bind(customer.id)
        .bind(hash)
        .execute(&mut *tx)
        .await
        .is_err()
        {
            return unavailable();
        }
        "customer.password_changed"
    } else {
        match remove.as_deref() {
            Some("google") if has_password && has_google => {
                if sqlx::query("DELETE FROM customer_google_identities WHERE customer_id=$1").bind(customer.id).execute(&mut *tx).await.is_err() || sqlx::query("UPDATE customer_accounts SET google_auto_link_disabled=true WHERE customer_id=$1").bind(customer.id).execute(&mut *tx).await.is_err(){return unavailable()}
                "customer.google_unlinked"
            }
            Some("password") if has_google && has_password => {
                if sqlx::query("UPDATE customer_accounts SET password_hash=NULL,updated_at=now() WHERE customer_id=$1").bind(customer.id).execute(&mut *tx).await.is_err(){return unavailable()}
                "customer.password_removed"
            }
            _ => return failure("last_signin_method"),
        }
    };
    let token = new_session_token();
    if invalidate(&mut tx, customer.id).await.is_err()
        || account::insert_session(&mut tx, Uuid::now_v7(), customer.id, &hash_token(&token))
            .await
            .is_err()
        || account::audit(&mut tx, customer.id, action, "customer", customer.id)
            .await
            .is_err()
        || tx.commit().await.is_err()
    {
        return unavailable();
    }
    let _ = state
        .email
        .send_security_notice(&customer.email, action)
        .await;
    match profile(pool, customer.id).await {
        Ok(profile) => no_store(
            (
                jar.add(session_cookie(token, state.secure_cookies)),
                Json(profile),
            )
                .into_response(),
        ),
        Err(_) => unavailable(),
    }
}
fn failure(code: &'static str) -> Response {
    no_store(error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        code,
        match code {
            "google_email_locked" => "Set a password and unlink Google before changing your email.",
            "google_email_mismatch" => {
                "Your Google email no longer matches. Use password recovery, then unlink and reconnect Google."
            }
            "reauthentication_required" => "Confirm your sign-in method before continuing.",
            "last_signin_method" => "Keep at least one sign-in method connected.",
            _ => "Unable to confirm this sign-in request. Please try again.",
        },
    ))
}
fn unavailable() -> Response {
    no_store(error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "account_unavailable",
        "Account service temporarily unavailable.",
    ))
}
fn limited() -> Response {
    no_store(error_response(
        StatusCode::TOO_MANY_REQUESTS,
        "account_action_limited",
        "Too many attempts. Please try again later.",
    ))
}

#[cfg(test)]
mod tests;
