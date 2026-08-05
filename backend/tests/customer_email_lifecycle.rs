use std::{env, str::FromStr};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use knitprint_api::{AppState, app, email::EmailService};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

const CUSTOMER_COOKIE: &str = "knitprint_customer";
const OLD_PASSWORD: &str = "old-customer-passphrase";
const NEW_PASSWORD: &str = "new-customer-passphrase";

#[tokio::test]
async fn verification_and_password_recovery_tokens_are_private_single_use_and_session_safe() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("customer_email_test_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("test database should be available");
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&admin)
        .await
        .expect("test schema should be created");
    let pool = isolated_pool(&database_url, &schema).await;
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("migrations should run in the isolated schema");
    let email = "email-lifecycle@example.test";
    let router = app(AppState {
        database: Some(pool.clone()),
        email: EmailService::development("http://127.0.0.1:3000"),
        ..AppState::default()
    });

    let registered = request(
        &router,
        "POST",
        "/api/account/register",
        None,
        Some(json!({
            "email": email,
            "password": OLD_PASSWORD,
            "first_name": "Marta",
            "last_name": "Silva"
        })),
    )
    .await;
    assert_eq!(registered.status(), StatusCode::CREATED);
    let cookie = response_cookie(&registered, CUSTOMER_COOKIE);
    let profile = response_json(registered).await;
    assert_eq!(profile["email_verified"], false);
    let customer_id = Uuid::parse_str(profile["id"].as_str().unwrap()).unwrap();

    let verification_email = latest_email(&router, email, "email_verification").await;
    assert_eq!(verification_email["subject"], "Verify your KnitPrint email");
    let verification_token = action_token(&verification_email, "verify");
    assert_token_is_only_hashed(&pool, &verification_token, "email_verification").await;

    let suppressed_resend = request(
        &router,
        "POST",
        "/api/account/verification/request",
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(suppressed_resend.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        action_token(
            &latest_email(&router, email, "email_verification").await,
            "verify"
        ),
        verification_token
    );
    let active_verification_tokens: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM customer_account_tokens WHERE customer_id = $1 AND token_kind = 'email_verification' AND used_at IS NULL",
    )
    .bind(customer_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active_verification_tokens, 1);

    let invalid_verification = request(
        &router,
        "POST",
        "/api/account/verification/confirm",
        None,
        Some(json!({ "token": "0".repeat(64) })),
    )
    .await;
    assert_eq!(
        invalid_verification.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let confirmed = request(
        &router,
        "POST",
        "/api/account/verification/confirm",
        None,
        Some(json!({ "token": verification_token })),
    )
    .await;
    assert_eq!(confirmed.status(), StatusCode::NO_CONTENT);
    let verified_profile = request(&router, "GET", "/api/account/me", Some(&cookie), None).await;
    assert_eq!(verified_profile.status(), StatusCode::OK);
    assert_eq!(
        response_json(verified_profile).await["email_verified"],
        true
    );
    let reused_verification = request(
        &router,
        "POST",
        "/api/account/verification/confirm",
        None,
        Some(json!({ "token": verification_token })),
    )
    .await;
    assert_eq!(
        reused_verification.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let unknown_forgot = request(
        &router,
        "POST",
        "/api/account/password/forgot",
        None,
        Some(json!({ "email": "unknown@example.test" })),
    )
    .await;
    assert_eq!(unknown_forgot.status(), StatusCode::NO_CONTENT);
    let unknown_mailbox = request(
        &router,
        "GET",
        "/api/development/emails/latest?to=unknown%40example.test&kind=password_reset",
        None,
        None,
    )
    .await;
    assert_eq!(unknown_mailbox.status(), StatusCode::NOT_FOUND);

    let forgot = request(
        &router,
        "POST",
        "/api/account/password/forgot",
        None,
        Some(json!({ "email": email })),
    )
    .await;
    assert_eq!(forgot.status(), StatusCode::NO_CONTENT);
    let reset_email = latest_email(&router, email, "password_reset").await;
    assert_eq!(reset_email["subject"], "Reset your KnitPrint password");
    let reset_token = action_token(&reset_email, "reset");
    assert_token_is_only_hashed(&pool, &reset_token, "password_reset").await;
    let repeated_forgot = request(
        &router,
        "POST",
        "/api/account/password/forgot",
        None,
        Some(json!({ "email": email })),
    )
    .await;
    assert_eq!(repeated_forgot.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        action_token(
            &latest_email(&router, email, "password_reset").await,
            "reset"
        ),
        reset_token
    );

    let weak_reset = request(
        &router,
        "POST",
        "/api/account/password/reset",
        None,
        Some(json!({ "token": reset_token, "password": "short" })),
    )
    .await;
    assert_eq!(weak_reset.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let reset = request(
        &router,
        "POST",
        "/api/account/password/reset",
        Some(&cookie),
        Some(json!({ "token": reset_token, "password": NEW_PASSWORD })),
    )
    .await;
    assert_eq!(reset.status(), StatusCode::NO_CONTENT);
    assert!(
        reset
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .any(|value| value.starts_with(CUSTOMER_COOKIE) && value.contains("Max-Age=0"))
    );
    let revoked_session = request(&router, "GET", "/api/account/me", Some(&cookie), None).await;
    assert_eq!(revoked_session.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        login(&router, email, OLD_PASSWORD).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        login(&router, email, NEW_PASSWORD).await.status(),
        StatusCode::OK
    );
    let reused_reset = request(
        &router,
        "POST",
        "/api/account/password/reset",
        None,
        Some(json!({ "token": reset_token, "password": OLD_PASSWORD })),
    )
    .await;
    assert_eq!(reused_reset.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let expiry_request = request(
        &router,
        "POST",
        "/api/account/password/forgot",
        None,
        Some(json!({ "email": email })),
    )
    .await;
    assert_eq!(expiry_request.status(), StatusCode::NO_CONTENT);
    let expired_token = action_token(
        &latest_email(&router, email, "password_reset").await,
        "reset",
    );
    let expired_hash = Sha256::digest(expired_token.as_bytes());
    sqlx::query(
        r#"
        UPDATE customer_account_tokens
        SET created_at = now() - interval '2 hours',
            expires_at = now() - interval '1 hour'
        WHERE token_hash = $1
        "#,
    )
    .bind(expired_hash.as_slice())
    .execute(&pool)
    .await
    .expect("reset token fixture should expire");
    let expired_reset = request(
        &router,
        "POST",
        "/api/account/password/reset",
        None,
        Some(json!({ "token": expired_token, "password": OLD_PASSWORD })),
    )
    .await;
    assert_eq!(expired_reset.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let actions: Vec<String> =
        sqlx::query_scalar("SELECT action FROM audit_log WHERE entity_id = $1 ORDER BY id")
            .bind(customer_id.to_string())
            .fetch_all(&pool)
            .await
            .expect("account email audits should be readable");
    assert!(actions.contains(&"customer.email_verified".into()));
    assert!(actions.contains(&"customer.password_reset_requested".into()));
    assert!(actions.contains(&"customer.password_reset".into()));

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .expect("test schema should be removed");
    admin.close().await;
}

async fn latest_email(router: &axum::Router, email: &str, kind: &str) -> Value {
    let path = format!(
        "/api/development/emails/latest?to={}&kind={kind}",
        email.replace('@', "%40")
    );
    let response = request(router, "GET", &path, None, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    response_json(response).await
}

fn action_token(email: &Value, parameter: &str) -> String {
    email["action_url"]
        .as_str()
        .unwrap()
        .split_once(&format!("{parameter}="))
        .unwrap()
        .1
        .to_owned()
}

async fn assert_token_is_only_hashed(pool: &PgPool, token: &str, kind: &str) {
    let stored: Vec<u8> = sqlx::query_scalar(
        "SELECT token_hash FROM customer_account_tokens WHERE token_kind = $1 AND used_at IS NULL",
    )
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("account token hash should be stored");
    assert_eq!(stored, Sha256::digest(token.as_bytes()).as_slice());
    assert_ne!(stored, token.as_bytes());
}

async fn login(router: &axum::Router, email: &str, password: &str) -> axum::response::Response {
    request(
        router,
        "POST",
        "/api/account/login",
        None,
        Some(json!({ "email": email, "password": password })),
    )
    .await
}

async fn request(
    router: &axum::Router,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    router
        .clone()
        .oneshot(
            builder
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
                .unwrap(),
        )
        .await
        .unwrap()
}

fn response_cookie(response: &axum::response::Response, name: &str) -> String {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(&format!("{name}=")))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn isolated_pool(database_url: &str, schema: &str) -> PgPool {
    let options = PgConnectOptions::from_str(database_url).unwrap();
    let search_path = format!(r#"SET search_path TO "{schema}", public"#);
    PgPoolOptions::new()
        .max_connections(3)
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query(&search_path).execute(connection).await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .unwrap()
}
