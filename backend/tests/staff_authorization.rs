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
async fn staff_authorization_and_audit_lifecycle() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("staff_test_{}", Uuid::new_v4().simple());
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

    let owner_id = insert_staff(&pool, "owner@test.invalid", "owner", &[]).await;
    insert_staff(&pool, "limited@test.invalid", "staff", &["catalog.read"]).await;
    let router = app(AppState {
        database: Some(pool.clone()),
        media_storage: None,
        secure_cookies: false,
    });

    let anonymous = request(&router, "GET", "/api/admin/staff", None, None).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    for _ in 0..5 {
        let rejected = request(
            &router,
            "POST",
            "/api/admin/auth/login",
            None,
            Some(json!({
                "email": "rate-limited@test.invalid",
                "password": "incorrect-integration-passphrase"
            })),
        )
        .await;
        assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);
    }
    let limited_login = request(
        &router,
        "POST",
        "/api/admin/auth/login",
        None,
        Some(json!({
            "email": "rate-limited@test.invalid",
            "password": "incorrect-integration-passphrase"
        })),
    )
    .await;
    assert_eq!(limited_login.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        limited_login.headers().get(header::RETRY_AFTER).unwrap(),
        "900"
    );

    let limited_cookie = login(&router, "limited@test.invalid").await;
    let forbidden = request(
        &router,
        "GET",
        "/api/admin/staff",
        Some(&limited_cookie),
        None,
    )
    .await;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    let owner_cookie = login(&router, "owner@test.invalid").await;
    let created = request(
        &router,
        "POST",
        "/api/admin/staff",
        Some(&owner_cookie),
        Some(json!({
            "email": "new-staff@test.invalid",
            "display_name": "New Staff",
            "password": "integration-test-passphrase",
            "capabilities": ["orders.read"]
        })),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created_body = response_json(created).await;
    let created_id = Uuid::parse_str(
        created_body["id"]
            .as_str()
            .expect("created staff response should contain an ID"),
    )
    .expect("created staff ID should be valid");
    let created_cookie = login(&router, "new-staff@test.invalid").await;

    let disabled = request(
        &router,
        "POST",
        &format!("/api/admin/staff/{created_id}/disable"),
        Some(&owner_cookie),
        Some(json!({ "reason": "Integration test access removal" })),
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::NO_CONTENT);

    let revoked = request(
        &router,
        "GET",
        "/api/admin/auth/me",
        Some(&created_cookie),
        None,
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);

    let audit: Vec<(String, Uuid, Option<String>)> = sqlx::query_as(
        r#"
        SELECT action, actor_staff_user_id, reason
        FROM audit_log
        WHERE entity_id = $1
        ORDER BY id
        "#,
    )
    .bind(created_id.to_string())
    .fetch_all(&pool)
    .await
    .expect("audit records should be readable");
    assert_eq!(
        audit,
        vec![
            ("staff.create".into(), owner_id, None),
            (
                "staff.disable".into(),
                owner_id,
                Some("Integration test access removal".into())
            ),
        ]
    );

    let forbidden_product = request(
        &router,
        "POST",
        "/api/admin/products",
        Some(&limited_cookie),
        Some(product_fixture("forbidden-product", "FORBIDDEN-001")),
    )
    .await;
    assert_eq!(forbidden_product.status(), StatusCode::FORBIDDEN);

    let created_product = request(
        &router,
        "POST",
        "/api/admin/products",
        Some(&owner_cookie),
        Some(product_fixture("woven-planter", "PLANTER-001")),
    )
    .await;
    assert_eq!(created_product.status(), StatusCode::CREATED);
    let product_body = response_json(created_product).await;
    let product_id = product_body["id"]
        .as_str()
        .expect("product should contain an ID");
    assert_eq!(product_body["status"], "draft");
    assert_eq!(product_body["variants"][0]["price_minor"], 4200);
    assert_eq!(product_body["variants"][0]["currency"], "EUR");

    let drafts_are_private = request(&router, "GET", "/api/products", None, None).await;
    assert_eq!(response_json(drafts_are_private).await, json!([]));

    let published = request(
        &router,
        "POST",
        &format!("/api/admin/products/{product_id}/status"),
        Some(&owner_cookie),
        Some(json!({ "status": "active" })),
    )
    .await;
    assert_eq!(published.status(), StatusCode::OK);

    let search = request(&router, "GET", "/api/products?q=stitch", None, None).await;
    let search_body = response_json(search).await;
    assert_eq!(search_body[0]["slug"], "woven-planter");

    let public_detail = request(&router, "GET", "/api/products/woven-planter", None, None).await;
    assert_eq!(public_detail.status(), StatusCode::OK);

    let archived = request(
        &router,
        "POST",
        &format!("/api/admin/products/{product_id}/status"),
        Some(&owner_cookie),
        Some(json!({ "status": "archived" })),
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    let archived_detail = request(&router, "GET", "/api/products/woven-planter", None, None).await;
    assert_eq!(archived_detail.status(), StatusCode::NOT_FOUND);

    let catalog_audit: Vec<(String, Uuid, Option<String>)> = sqlx::query_as(
        r#"
        SELECT action, actor_staff_user_id, reason
        FROM audit_log
        WHERE entity_id = $1
        ORDER BY id
        "#,
    )
    .bind(product_id)
    .fetch_all(&pool)
    .await
    .expect("catalog audit records should be readable");
    assert_eq!(
        catalog_audit,
        vec![
            ("product.create".into(), owner_id, None),
            (
                "product.status_change".into(),
                owner_id,
                Some("active".into())
            ),
            (
                "product.status_change".into(),
                owner_id,
                Some("archived".into())
            ),
        ]
    );

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .expect("test schema should be removed");
}

fn product_fixture(slug: &str, sku: &str) -> Value {
    json!({
        "title": "Woven Planter",
        "slug": slug,
        "description": "A tactile home for knitted and printed forms.",
        "search_keywords": "stitch yarn home",
        "variants": [{
            "title": "Natural",
            "sku": sku,
            "price_minor": 4200,
            "currency": "EUR",
            "option_values": { "colour": "Natural" }
        }]
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

async fn insert_staff(pool: &PgPool, email: &str, role: &str, capabilities: &[&str]) -> Uuid {
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
    id
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
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("login should set a session cookie")
        .to_str()
        .expect("session cookie should be valid");
    cookie
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
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        assert!(cookie.starts_with(SESSION_COOKIE));
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

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&bytes).expect("response should be JSON")
}
