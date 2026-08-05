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

#[tokio::test]
async fn guest_capture_and_private_customer_inspection_enforce_privacy_rules() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("customer_test_{}", Uuid::new_v4().simple());
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

    insert_staff(&pool, "owner@customer.test", "owner", &[]).await;
    insert_staff(
        &pool,
        "catalog-only@customer.test",
        "staff",
        &["catalog.read"],
    )
    .await;
    insert_staff(
        &pool,
        "customer-reader@customer.test",
        "staff",
        &["customers.read"],
    )
    .await;
    let router = app(AppState {
        database: Some(pool.clone()),
        ..AppState::default()
    });

    let invalid = request(
        &router,
        "POST",
        "/api/customers/guest",
        None,
        Some(json!({
            "email": "invalid",
            "first_name": "Ada",
            "last_name": "Lovelace",
            "address": {
                "recipient_name": "Ada Lovelace",
                "line1": "12 Loom Lane",
                "city": "Lisbon",
                "postal_code": "1000-001",
                "country_code": "PT"
            }
        })),
        Some("invalid-customer-fixture-001"),
    )
    .await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let missing_key = request(
        &router,
        "POST",
        "/api/customers/guest",
        None,
        Some(guest_fixture()),
        None,
    )
    .await;
    assert_eq!(missing_key.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let created = request(
        &router,
        "POST",
        "/api/customers/guest",
        None,
        Some(guest_fixture()),
        Some("guest-customer-fixture-001"),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    assert_eq!(created_body.as_object().unwrap().len(), 2);
    let customer_id = Uuid::parse_str(created_body["customer_id"].as_str().unwrap()).unwrap();
    let duplicate = request(
        &router,
        "POST",
        "/api/customers/guest",
        None,
        Some(guest_fixture()),
        Some("guest-customer-fixture-001"),
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::OK);
    assert_eq!(response_json(duplicate).await, created_body);
    let customer_count: i64 = sqlx::query_scalar("SELECT count(*) FROM customers")
        .fetch_one(&pool)
        .await
        .expect("customer count should be readable");
    assert_eq!(customer_count, 1);

    let stored: (String, String, String, String, String) = sqlx::query_as(
        r#"
        SELECT
            customer.email::text,
            customer.first_name,
            address.line1,
            address.country_code::text,
            customer.customer_type
        FROM customers customer
        JOIN customer_addresses address ON address.customer_id = customer.id
        WHERE customer.id = $1
        "#,
    )
    .bind(customer_id)
    .fetch_one(&pool)
    .await
    .expect("guest details should be stored");
    assert_eq!(
        stored,
        (
            "ada@example.com".into(),
            "Ada".into(),
            "12 Loom Lane".into(),
            "PT".into(),
            "guest".into(),
        )
    );

    let anonymous = request(&router, "GET", "/api/admin/customers", None, None, None).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    let limited_cookie = login(&router, "catalog-only@customer.test").await;
    let forbidden = request(
        &router,
        "GET",
        "/api/admin/customers",
        Some(&limited_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let reader_cookie = login(&router, "customer-reader@customer.test").await;
    let list = request(
        &router,
        "GET",
        "/api/admin/customers?q=lovelace",
        Some(&reader_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(list.status(), StatusCode::OK);
    let list_body = response_json(list).await;
    assert_eq!(list_body.as_array().unwrap().len(), 1);
    assert_eq!(list_body[0]["email"], "ada@example.com");
    assert_eq!(list_body[0]["address_count"], 1);

    let detail = request(
        &router,
        "GET",
        &format!("/api/admin/customers/{customer_id}"),
        Some(&reader_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = response_json(detail).await;
    assert_eq!(detail_body["phone"], "+351 210 000 000");
    assert_eq!(detail_body["addresses"][0]["city"], "Lisbon");
    assert_eq!(detail_body["order_count"], 0);

    let audit: Vec<(Option<Uuid>, String)> = sqlx::query_as(
        "SELECT actor_staff_user_id, action FROM audit_log WHERE entity_id = $1 ORDER BY id",
    )
    .bind(customer_id.to_string())
    .fetch_all(&pool)
    .await
    .expect("customer audit trail should be readable");
    assert_eq!(audit[0], (None, "customer.guest_create".into()));
    assert_eq!(audit[1].1, "customer.private_view");
    assert!(audit[1].0.is_some());
    let collection_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action = 'customer.private_list' AND entity_id IS NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("customer list audit should be readable");
    assert_eq!(collection_audits, 1);

    sqlx::query(
        "UPDATE customers SET retention_expires_at = now() - interval '1 day', created_at = now() - interval '2 days' WHERE id = $1",
    )
    .bind(customer_id)
    .execute(&pool)
    .await
    .expect("customer retention fixture should expire");
    let expired = request(
        &router,
        "GET",
        &format!("/api/admin/customers/{customer_id}"),
        Some(&reader_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(expired.status(), StatusCode::NOT_FOUND);
    let expired_list = request(
        &router,
        "GET",
        "/api/admin/customers?q=lovelace",
        Some(&reader_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(response_json(expired_list).await, json!([]));
    sqlx::query(
        "UPDATE customers SET retention_expires_at = now() + interval '1 day', anonymized_at = now() WHERE id = $1",
    )
    .bind(customer_id)
    .execute(&pool)
    .await
    .expect("customer anonymization fixture should be marked");
    let anonymized = request(
        &router,
        "GET",
        &format!("/api/admin/customers/{customer_id}"),
        Some(&reader_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(anonymized.status(), StatusCode::NOT_FOUND);

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .expect("test schema should be removed");
    admin.close().await;
}

fn guest_fixture() -> Value {
    json!({
        "email": " ADA@Example.COM ",
        "first_name": " Ada ",
        "last_name": " Lovelace ",
        "phone": "+351 210 000 000",
        "address": {
            "recipient_name": "Ada Lovelace",
            "line1": "12 Loom Lane",
            "line2": "Studio 4",
            "city": "Lisbon",
            "region": "Lisbon",
            "postal_code": "1000-001",
            "country_code": "pt",
            "phone": "+351 210 000 000"
        }
    })
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

async fn insert_staff(pool: &PgPool, email: &str, role: &str, capabilities: &[&str]) {
    let id = Uuid::now_v7();
    let password_hash = hash_password("integration-test-passphrase").expect("password should hash");
    sqlx::query(
        "INSERT INTO staff_users (id, email, display_name, password_hash, role) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(email)
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .execute(pool)
    .await
    .expect("staff fixture should be inserted");
    for capability in capabilities {
        sqlx::query(
            "INSERT INTO staff_capabilities (staff_user_id, capability_name) VALUES ($1, $2)",
        )
        .bind(id)
        .bind(capability)
        .execute(pool)
        .await
        .expect("capability fixture should be inserted");
    }
}

async fn login(router: &axum::Router, email: &str) -> String {
    let response = request(
        router,
        "POST",
        "/api/admin/auth/login",
        None,
        Some(json!({
            "email": email,
            "password": "integration-test-passphrase"
        })),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response
        .headers()
        .get(header::SET_COOKIE)
        .expect("login should set a session cookie")
        .to_str()
        .expect("session cookie should be valid")
        .split(';')
        .next()
        .expect("session cookie should contain a value")
        .to_owned()
}

async fn request(
    router: &axum::Router,
    method: &str,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
    idempotency_key: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        assert!(cookie.starts_with(SESSION_COOKIE));
        builder = builder.header(header::COOKIE, cookie);
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(idempotency_key) = idempotency_key {
        builder = builder.header("idempotency-key", idempotency_key);
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

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response should be JSON")
}
