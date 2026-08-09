use std::{env, str::FromStr};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use knitprint_api::{AppState, app, auth::hash_password};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn checkout_snapshots_reserves_and_manual_payment_are_idempotent() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("order_test_{}", Uuid::new_v4().simple());
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
        .expect("migrations should run");
    let variant_id = insert_product(&pool).await;
    insert_owner(&pool).await;
    let router = app(AppState {
        database: Some(pool.clone()),
        manual_payments_enabled: true,
        ..AppState::default()
    });

    let cart = request(&router, "GET", "/api/cart", None, None, None).await;
    let cart_cookie = response_cookie(&cart);
    let added = request(
        &router,
        "POST",
        "/api/cart/items",
        Some(&cart_cookie),
        Some(json!({ "variant_id": variant_id, "quantity": 2 })),
        Some("order-cart-add-0001"),
    )
    .await;
    assert_eq!(added.status(), StatusCode::CREATED);
    let delivered = request(
        &router,
        "POST",
        "/api/cart/delivery",
        Some(&cart_cookie),
        Some(delivery_fixture()),
        Some("order-delivery-0001"),
    )
    .await;
    assert_eq!(delivered.status(), StatusCode::OK);

    let created = request(
        &router,
        "POST",
        "/api/orders",
        Some(&cart_cookie),
        Some(json!({ "payment_method": "manual" })),
        Some("order-checkout-0001"),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    assert_eq!(
        created.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    let created_body = response_json(created).await;
    assert_eq!(created_body["order_status"], "pending");
    assert_eq!(created_body["payment_status"], "pending");
    assert_eq!(created_body["total_minor"], 6400);
    assert_eq!(created_body["lines"][0]["product_title"], "Order Loom");
    assert_eq!(created_body["shipping_address"]["line1"], "9 Thread Street");
    let order_id = created_body["id"].as_str().unwrap();

    let replay = request(
        &router,
        "POST",
        "/api/orders",
        Some(&cart_cookie),
        Some(json!({ "payment_method": "manual" })),
        Some("order-checkout-0001"),
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(response_json(replay).await["id"], order_id);
    let order_count: i64 = sqlx::query_scalar("SELECT count(*) FROM orders")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(order_count, 1);
    let quantities: (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, committed_quantity FROM inventory_items WHERE variant_id = $1",
    )
    .bind(variant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quantities, (3, 2, 0));

    sqlx::query("UPDATE products SET title = 'Renamed product' WHERE id = (SELECT product_id FROM product_variants WHERE id = $1)")
        .bind(variant_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE product_variants SET price_minor = 9999, sku = 'RENAMED' WHERE id = $1")
        .bind(variant_id)
        .execute(&pool)
        .await
        .unwrap();

    let owner_cookie = login(&router).await;
    let listed = request(
        &router,
        "GET",
        "/api/admin/orders",
        Some(&owner_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(response_json(listed).await[0]["item_count"], 2);
    let detail = request(
        &router,
        "GET",
        &format!("/api/admin/orders/{order_id}"),
        Some(&owner_cookie),
        None,
        None,
    )
    .await;
    let detail_body = response_json(detail).await;
    assert_eq!(detail_body["lines"][0]["product_title"], "Order Loom");
    assert_eq!(detail_body["lines"][0]["unit_price_minor"], 3200);

    let paid = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/manual-payment"),
        Some(&owner_cookie),
        Some(json!({ "reason": "Development payment received" })),
        None,
    )
    .await;
    assert_eq!(paid.status(), StatusCode::OK);
    let paid_body = response_json(paid).await;
    assert_eq!(paid_body["order_status"], "confirmed");
    assert_eq!(paid_body["payment_status"], "paid");
    assert_eq!(paid_body["timeline"].as_array().unwrap().len(), 2);
    let paid_replay = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/manual-payment"),
        Some(&owner_cookie),
        Some(json!({ "reason": "Development payment received" })),
        None,
    )
    .await;
    assert_eq!(paid_replay.status(), StatusCode::OK);
    let quantities: (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, committed_quantity FROM inventory_items WHERE variant_id = $1",
    )
    .bind(variant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quantities, (3, 0, 2));
    assert!(
        sqlx::query("UPDATE order_lines SET product_title = 'Tampered' WHERE order_id = $1")
            .bind(Uuid::parse_str(order_id).unwrap())
            .execute(&pool)
            .await
            .is_err(),
        "commercial snapshots must be immutable"
    );

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

async fn insert_product(pool: &PgPool) -> Uuid {
    let product_id = Uuid::now_v7();
    let variant_id = Uuid::now_v7();
    sqlx::query("INSERT INTO products (id, title, slug, description, status, published_at) VALUES ($1, 'Order Loom', 'order-loom', 'Fixture', 'active', now())")
        .bind(product_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price_minor, currency) VALUES ($1, $2, 'Mauve', 'ORDER-MAUVE', 3200, 'EUR')")
        .bind(variant_id)
        .bind(product_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE inventory_items SET available_quantity = 5 WHERE variant_id = $1")
        .bind(variant_id)
        .execute(pool)
        .await
        .unwrap();
    variant_id
}

async fn insert_owner(pool: &PgPool) {
    let password_hash = hash_password("integration-test-passphrase").unwrap();
    sqlx::query("INSERT INTO staff_users (id, email, display_name, password_hash, role) VALUES ($1, 'orders-owner@test.invalid', 'Order Owner', $2, 'owner')")
        .bind(Uuid::now_v7())
        .bind(password_hash)
        .execute(pool)
        .await
        .unwrap();
}

async fn login(router: &axum::Router) -> String {
    let response = request(
        router,
        "POST",
        "/api/admin/auth/login",
        None,
        Some(json!({
            "email": "orders-owner@test.invalid",
            "password": "integration-test-passphrase"
        })),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response_cookie(&response)
}

fn delivery_fixture() -> Value {
    json!({
        "email": "order@example.com",
        "first_name": "Ada",
        "last_name": "Loom",
        "phone": "+351 210 000 000",
        "address": {
            "recipient_name": "Ada Loom",
            "line1": "9 Thread Street",
            "line2": "",
            "city": "Lisbon",
            "region": "Lisbon",
            "postal_code": "1000-009",
            "country_code": "PT",
            "phone": "+351 210 000 000"
        }
    })
}

async fn isolated_pool(database_url: &str, schema: &str) -> PgPool {
    let options = PgConnectOptions::from_str(database_url).unwrap();
    let search_path = format!(r#"SET search_path TO "{schema}", public"#);
    PgPoolOptions::new()
        .max_connections(4)
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
        builder = builder.header(header::COOKIE, cookie);
    }
    if body.is_some() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(key) = idempotency_key {
        builder = builder.header("idempotency-key", key);
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

fn response_cookie(response: &axum::response::Response) -> String {
    response.headers()[header::SET_COOKIE]
        .to_str()
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
