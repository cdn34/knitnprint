use std::{env, str::FromStr};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use knitprint_api::{
    AppState, app,
    auth::{SESSION_COOKIE, hash_password},
};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

const CUSTOMER_SESSION_COOKIE: &str = "knitprint_customer";
const CUSTOMER_PASSWORD: &str = "integration-customer-passphrase";

#[tokio::test]
async fn customer_account_authentication_and_address_ownership_lifecycle() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("customer_account_test_{}", Uuid::new_v4().simple());
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

    insert_staff(&pool, "owner@account.test").await;
    let router = app(AppState {
        database: Some(pool.clone()),
        email: knitprint_api::email::EmailService::development("http://127.0.0.1:3000"),
        ..AppState::default()
    });

    let anonymous_me = request(&router, "GET", "/api/account/me", None, None).await;
    assert_eq!(anonymous_me.status(), StatusCode::UNAUTHORIZED);
    let anonymous_address = request(
        &router,
        "POST",
        "/api/account/addresses",
        None,
        Some(address_fixture()),
    )
    .await;
    assert_eq!(anonymous_address.status(), StatusCode::UNAUTHORIZED);

    let invalid_email = request(
        &router,
        "POST",
        "/api/account/register",
        None,
        Some(json!({
            "email": "not-an-email",
            "password": CUSTOMER_PASSWORD,
            "first_name": "Ada",
            "last_name": "Lovelace"
        })),
    )
    .await;
    assert_eq!(invalid_email.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let weak_password = request(
        &router,
        "POST",
        "/api/account/register",
        None,
        Some(json!({
            "email": "weak-password@example.com",
            "password": "short",
            "first_name": "Ada",
            "last_name": "Lovelace"
        })),
    )
    .await;
    assert_eq!(weak_password.status(), StatusCode::UNPROCESSABLE_ENTITY);

    for _ in 0..5 {
        let failed_login = request(
            &router,
            "POST",
            "/api/account/login",
            None,
            Some(json!({
                "email": "rate-limited@example.com",
                "password": "incorrect-customer-passphrase"
            })),
        )
        .await;
        assert_eq!(failed_login.status(), StatusCode::UNAUTHORIZED);
    }
    let limited_login = request(
        &router,
        "POST",
        "/api/account/login",
        None,
        Some(json!({
            "email": "rate-limited@example.com",
            "password": "incorrect-customer-passphrase"
        })),
    )
    .await;
    assert_eq!(limited_login.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        limited_login.headers().get(header::RETRY_AFTER),
        Some(&header::HeaderValue::from_static("900"))
    );

    let registered = request(
        &router,
        "POST",
        "/api/account/register",
        None,
        Some(registration_fixture(
            " ADA.Account@Example.COM ",
            "Ada",
            "Lovelace",
        )),
    )
    .await;
    assert_eq!(registered.status(), StatusCode::CREATED);
    assert_private_no_store(&registered);
    assert_development_customer_cookie(&registered);
    let customer_cookie = response_cookie(&registered, CUSTOMER_SESSION_COOKIE);
    let raw_customer_token = cookie_value(&customer_cookie);
    let registered_body = response_json(registered).await;
    let customer_id = profile_id(&registered_body);
    assert_eq!(registered_body["email"], "ada.account@example.com");
    assert_eq!(registered_body["first_name"], "Ada");
    assert_eq!(registered_body["last_name"], "Lovelace");
    assert_eq!(registered_body["phone"], "+351 210 000 001");
    assert_eq!(registered_body["addresses"], json!([]));

    let duplicate = request(
        &router,
        "POST",
        "/api/account/register",
        None,
        Some(registration_fixture(
            "ada.account@example.com",
            "Different",
            "Person",
        )),
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);

    let account_row: (String, String, String) = sqlx::query_as(
        r#"
        SELECT customer.email::text, customer.customer_type, account.password_hash
        FROM customer_accounts account
        JOIN customers customer ON customer.id = account.customer_id
        WHERE account.customer_id = $1
        "#,
    )
    .bind(customer_id)
    .fetch_one(&pool)
    .await
    .expect("registered account should be stored");
    assert_eq!(account_row.0, "ada.account@example.com");
    assert_eq!(account_row.1, "registered");
    assert_ne!(account_row.2, CUSTOMER_PASSWORD);
    assert!(account_row.2.starts_with("$argon2"));

    let session_hash: Vec<u8> = sqlx::query_scalar(
        "SELECT token_hash FROM customer_sessions WHERE customer_id = $1 AND revoked_at IS NULL",
    )
    .bind(customer_id)
    .fetch_one(&pool)
    .await
    .expect("registration should create a session");
    assert_eq!(session_hash.len(), 32);
    assert_ne!(session_hash, raw_customer_token.as_bytes());
    let raw_token_match: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM customer_sessions WHERE token_hash = convert_to($1, 'UTF8')",
    )
    .bind(&raw_customer_token)
    .fetch_one(&pool)
    .await
    .expect("session hashes should be inspectable");
    assert_eq!(
        raw_token_match, 0,
        "raw session tokens must never be stored"
    );

    let me = request(
        &router,
        "GET",
        "/api/account/me",
        Some(&customer_cookie),
        None,
    )
    .await;
    assert_eq!(me.status(), StatusCode::OK);
    assert_private_no_store(&me);
    assert_eq!(response_json(me).await, registered_body);

    let admin_cookie = admin_login(&router, "owner@account.test").await;
    assert!(admin_cookie.starts_with(&format!("{SESSION_COOKIE}=")));
    let admin_is_not_customer =
        request(&router, "GET", "/api/account/me", Some(&admin_cookie), None).await;
    assert_eq!(admin_is_not_customer.status(), StatusCode::UNAUTHORIZED);
    let customer_is_not_admin = request(
        &router,
        "GET",
        "/api/admin/auth/me",
        Some(&customer_cookie),
        None,
    )
    .await;
    assert_eq!(customer_is_not_admin.status(), StatusCode::UNAUTHORIZED);

    let created_address = request(
        &router,
        "POST",
        "/api/account/addresses",
        Some(&customer_cookie),
        Some(address_fixture()),
    )
    .await;
    assert_eq!(created_address.status(), StatusCode::CREATED);
    assert_private_no_store(&created_address);
    let address_body = response_json(created_address).await;
    let address_id = Uuid::parse_str(
        address_body["id"]
            .as_str()
            .expect("address response should contain an ID"),
    )
    .expect("address ID should be valid");
    assert_eq!(address_body["address_type"], "delivery");
    assert_eq!(address_body["recipient_name"], "Ada Lovelace");
    assert_eq!(address_body["city"], "Lisbon");
    assert_eq!(address_body["country_code"], "PT");
    let address_owner: Uuid =
        sqlx::query_scalar("SELECT customer_id FROM customer_addresses WHERE id = $1")
            .bind(address_id)
            .fetch_one(&pool)
            .await
            .expect("address owner should be stored");
    assert_eq!(address_owner, customer_id);

    let me_with_address = request(
        &router,
        "GET",
        "/api/account/me",
        Some(&customer_cookie),
        None,
    )
    .await;
    assert_eq!(me_with_address.status(), StatusCode::OK);
    assert_private_no_store(&me_with_address);
    let me_with_address_body = response_json(me_with_address).await;
    assert_eq!(me_with_address_body["addresses"], json!([address_body]));

    let other_registration = request(
        &router,
        "POST",
        "/api/account/register",
        None,
        Some(registration_fixture(
            "grace.account@example.com",
            "Grace",
            "Hopper",
        )),
    )
    .await;
    assert_eq!(other_registration.status(), StatusCode::CREATED);
    assert_private_no_store(&other_registration);
    assert_development_customer_cookie(&other_registration);
    let other_cookie = response_cookie(&other_registration, CUSTOMER_SESSION_COOKIE);
    let other_me = request(&router, "GET", "/api/account/me", Some(&other_cookie), None).await;
    assert_eq!(other_me.status(), StatusCode::OK);
    assert_private_no_store(&other_me);
    let other_body = response_json(other_me).await;
    assert_eq!(other_body["email"], "grace.account@example.com");
    assert_eq!(other_body["addresses"], json!([]));

    let logout = request(
        &router,
        "POST",
        "/api/account/logout",
        Some(&customer_cookie),
        None,
    )
    .await;
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);
    assert_private_no_store(&logout);
    assert!(
        logout
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .any(|value| value.starts_with(&format!("{CUSTOMER_SESSION_COOKIE}="))),
        "logout should remove the customer cookie"
    );
    let revoked_me = request(
        &router,
        "GET",
        "/api/account/me",
        Some(&customer_cookie),
        None,
    )
    .await;
    assert_eq!(revoked_me.status(), StatusCode::UNAUTHORIZED);
    let was_revoked: bool = sqlx::query_scalar(
        "SELECT revoked_at IS NOT NULL FROM customer_sessions WHERE customer_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(customer_id)
    .fetch_one(&pool)
    .await
    .expect("revoked session should remain auditable");
    assert!(was_revoked);

    let bad_login = request(
        &router,
        "POST",
        "/api/account/login",
        None,
        Some(json!({
            "email": "ada.account@example.com",
            "password": "incorrect-customer-passphrase"
        })),
    )
    .await;
    assert_eq!(bad_login.status(), StatusCode::UNAUTHORIZED);
    let login = request(
        &router,
        "POST",
        "/api/account/login",
        None,
        Some(json!({
            "email": " ADA.Account@Example.COM ",
            "password": CUSTOMER_PASSWORD
        })),
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    assert_private_no_store(&login);
    assert_development_customer_cookie(&login);
    let login_cookie = response_cookie(&login, CUSTOMER_SESSION_COOKIE);
    let login_body = response_json(login).await;
    assert_eq!(profile_id(&login_body), customer_id);
    assert_eq!(login_body["addresses"].as_array().unwrap().len(), 1);

    sqlx::query(
        r#"
        UPDATE customer_sessions
        SET created_at = now() - interval '2 days',
            expires_at = now() - interval '1 day'
        WHERE token_hash = (
            SELECT token_hash
            FROM customer_sessions
            WHERE customer_id = $1 AND revoked_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
        )
        "#,
    )
    .bind(customer_id)
    .execute(&pool)
    .await
    .expect("active session fixture should expire");
    let expired_me = request(&router, "GET", "/api/account/me", Some(&login_cookie), None).await;
    assert_eq!(expired_me.status(), StatusCode::UNAUTHORIZED);

    let relogin = request(
        &router,
        "POST",
        "/api/account/login",
        None,
        Some(json!({
            "email": "ada.account@example.com",
            "password": CUSTOMER_PASSWORD
        })),
    )
    .await;
    assert_eq!(relogin.status(), StatusCode::OK);
    assert_private_no_store(&relogin);
    assert_development_customer_cookie(&relogin);
    let disabled_cookie = response_cookie(&relogin, CUSTOMER_SESSION_COOKIE);
    sqlx::query("UPDATE customer_accounts SET disabled_at = now() WHERE customer_id = $1")
        .bind(customer_id)
        .execute(&pool)
        .await
        .expect("account fixture should be disabled");
    let disabled_me = request(
        &router,
        "GET",
        "/api/account/me",
        Some(&disabled_cookie),
        None,
    )
    .await;
    assert_eq!(disabled_me.status(), StatusCode::UNAUTHORIZED);
    let disabled_login = request(
        &router,
        "POST",
        "/api/account/login",
        None,
        Some(json!({
            "email": "ada.account@example.com",
            "password": CUSTOMER_PASSWORD
        })),
    )
    .await;
    assert_eq!(disabled_login.status(), StatusCode::UNAUTHORIZED);

    let audit: Vec<(Option<Uuid>, String, String)> = sqlx::query_as(
        r#"
        SELECT actor_customer_id, action, entity_type
        FROM audit_log
        WHERE actor_customer_id = $1
        ORDER BY id
        "#,
    )
    .bind(customer_id)
    .fetch_all(&pool)
    .await
    .expect("customer account audit trail should be readable");
    assert_eq!(
        audit,
        vec![
            (
                Some(customer_id),
                "customer.register".into(),
                "customer".into()
            ),
            (
                Some(customer_id),
                "customer.address_create".into(),
                "customer_address".into()
            ),
            (
                Some(customer_id),
                "customer.logout".into(),
                "customer_session".into()
            ),
            (
                Some(customer_id),
                "customer.login".into(),
                "customer_session".into()
            ),
            (
                Some(customer_id),
                "customer.login".into(),
                "customer_session".into()
            ),
        ]
    );

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .expect("test schema should be removed");
    admin.close().await;
}

fn registration_fixture(email: &str, first_name: &str, last_name: &str) -> Value {
    json!({
        "email": email,
        "password": CUSTOMER_PASSWORD,
        "first_name": first_name,
        "last_name": last_name,
        "phone": "+351 210 000 001"
    })
}

fn address_fixture() -> Value {
    json!({
        "address_type": "delivery",
        "recipient_name": " Ada Lovelace ",
        "line1": " 12 Loom Lane ",
        "line2": "Studio 4",
        "city": " Lisbon ",
        "region": "Lisbon",
        "postal_code": "1000-001",
        "country_code": "pt",
        "phone": "+351 210 000 001"
    })
}

fn profile_id(profile: &Value) -> Uuid {
    Uuid::parse_str(
        profile["id"]
            .as_str()
            .expect("account profile should contain an ID"),
    )
    .expect("account profile ID should be valid")
}

async fn isolated_pool(database_url: &str, schema: &str) -> PgPool {
    let options = PgConnectOptions::from_str(database_url).expect("DATABASE_URL should be valid");
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
        .expect("isolated test pool should connect")
}

async fn insert_staff(pool: &PgPool, email: &str) {
    let password_hash = hash_password("integration-test-passphrase").expect("password should hash");
    sqlx::query(
        "INSERT INTO staff_users (id, email, display_name, password_hash, role) VALUES ($1, $2, $3, $4, 'owner')",
    )
    .bind(Uuid::now_v7())
    .bind(email)
    .bind(email)
    .bind(password_hash)
    .execute(pool)
    .await
    .expect("staff fixture should be inserted");
}

async fn admin_login(router: &axum::Router, email: &str) -> String {
    let response = request(
        router,
        "POST",
        "/api/admin/auth/login",
        None,
        Some(json!({
            "email": email,
            "password": "integration-test-passphrase"
        })),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_cookie(&response, SESSION_COOKIE)
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
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
}

fn response_cookie(response: &axum::response::Response, name: &str) -> String {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(&format!("{name}=")))
        .expect("response should set the expected session cookie")
        .split(';')
        .next()
        .expect("session cookie should contain a value")
        .to_owned()
}

fn assert_private_no_store(response: &axum::response::Response) {
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&header::HeaderValue::from_static("no-store, private")),
        "customer account responses containing private data must not be cached"
    );
}

fn assert_development_customer_cookie(response: &axum::response::Response) {
    let cookie = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find(|value| value.starts_with(&format!("{CUSTOMER_SESSION_COOKIE}=")))
        .expect("response should set the customer session cookie");
    let attributes = cookie.to_ascii_lowercase();
    assert!(attributes.contains("; httponly"));
    assert!(attributes.contains("; samesite=lax"));
    assert!(attributes.contains("; path=/api"));
    assert!(
        !attributes
            .split(';')
            .any(|attribute| attribute.trim() == "secure"),
        "development customer cookies should not require HTTPS"
    );
}

fn cookie_value(cookie: &str) -> String {
    cookie
        .split_once('=')
        .expect("session cookie should contain a name and value")
        .1
        .to_owned()
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response should be JSON")
}
