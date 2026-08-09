use std::{env, str::FromStr, sync::Arc};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use hmac::{Hmac, Mac};
use knitprint_api::{
    AppState, app,
    auth::hash_password,
    payments::{
        PaymentProvider, PaymentProviderError, PaymentService, ProviderCheckout,
        ProviderCheckoutRequest, ProviderFuture,
    },
};
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

struct FakeStripe;

impl PaymentProvider for FakeStripe {
    fn create_checkout(&self, request: ProviderCheckoutRequest) -> ProviderFuture<'_> {
        Box::pin(async move {
            Ok::<_, PaymentProviderError>(ProviderCheckout {
                provider_payment_id: format!("cs_test_{}", request.order_id.simple()),
                checkout_url: format!("https://checkout.stripe.test/{}", request.order_id),
                expires_at: time::OffsetDateTime::now_utc().unix_timestamp() + 1800,
            })
        })
    }
}

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

#[tokio::test]
async fn stripe_webhooks_are_signed_idempotent_and_drive_inventory() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("stripe_order_test_{}", Uuid::new_v4().simple());
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
    let webhook_secret = "whsec_order_lifecycle";
    let router = app(AppState {
        database: Some(pool.clone()),
        payments: PaymentService::with_provider(Arc::new(FakeStripe), webhook_secret),
        ..AppState::default()
    });

    let (cart_cookie, order_id) = create_stripe_order(&router, variant_id, "paid", None).await;
    let checkout = request(
        &router,
        "POST",
        &format!("/api/orders/{order_id}/payment"),
        Some(&cart_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(checkout.status(), StatusCode::OK);
    assert!(
        response_json(checkout).await["checkout_url"]
            .as_str()
            .unwrap()
            .starts_with("https://checkout.stripe.test/")
    );

    let session_id = format!("cs_test_{}", Uuid::parse_str(&order_id).unwrap().simple());
    let paid_event = stripe_event(
        "evt_paid",
        "checkout.session.completed",
        &session_id,
        &order_id,
        "paid",
    );
    assert_eq!(
        webhook_request(&router, webhook_secret, &paid_event)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        webhook_request(&router, webhook_secret, &paid_event)
            .await
            .status(),
        StatusCode::OK
    );
    let late_failure = stripe_event(
        "evt_late_failure",
        "checkout.session.async_payment_failed",
        &session_id,
        &order_id,
        "unpaid",
    );
    assert_eq!(
        webhook_request(&router, webhook_secret, &late_failure)
            .await
            .status(),
        StatusCode::OK
    );
    let paid_order = request(
        &router,
        "GET",
        &format!("/api/orders/{order_id}"),
        Some(&cart_cookie),
        None,
        None,
    )
    .await;
    let paid_body = response_json(paid_order).await;
    assert_eq!(paid_body["order_status"], "confirmed");
    assert_eq!(paid_body["payment_status"], "paid");
    assert_eq!(paid_body["payment"]["attempts"][0]["status"], "succeeded");
    assert_eq!(paid_body["payment"]["history"].as_array().unwrap().len(), 3);
    let quantities: (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, committed_quantity FROM inventory_items WHERE variant_id = $1",
    )
    .bind(variant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quantities, (3, 0, 2));

    sqlx::query("UPDATE inventory_items SET available_quantity = available_quantity + 2 WHERE variant_id = $1")
        .bind(variant_id)
        .execute(&pool)
        .await
        .unwrap();
    let fresh_cart = request(&router, "GET", "/api/cart", Some(&cart_cookie), None, None).await;
    let fresh_cookie = response_cookie(&fresh_cart);
    let (expired_cookie, expired_order_id) =
        create_stripe_order(&router, variant_id, "expired", Some(fresh_cookie)).await;
    let expired_checkout = request(
        &router,
        "POST",
        &format!("/api/orders/{expired_order_id}/payment"),
        Some(&expired_cookie),
        None,
        None,
    )
    .await;
    assert_eq!(expired_checkout.status(), StatusCode::OK);
    let expired_session = format!(
        "cs_test_{}",
        Uuid::parse_str(&expired_order_id).unwrap().simple()
    );
    let expired_event = stripe_event(
        "evt_expired",
        "checkout.session.expired",
        &expired_session,
        &expired_order_id,
        "unpaid",
    );
    assert_eq!(
        webhook_request(&router, webhook_secret, &expired_event)
            .await
            .status(),
        StatusCode::OK
    );
    let expired_order = request(
        &router,
        "GET",
        &format!("/api/orders/{expired_order_id}"),
        Some(&expired_cookie),
        None,
        None,
    )
    .await;
    let expired_body = response_json(expired_order).await;
    assert_eq!(expired_body["order_status"], "cancelled");
    assert_eq!(expired_body["payment_status"], "failed");
    let quantities: (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, committed_quantity FROM inventory_items WHERE variant_id = $1",
    )
    .bind(variant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quantities, (5, 0, 2));

    let third_cart = request(
        &router,
        "GET",
        "/api/cart",
        Some(&expired_cookie),
        None,
        None,
    )
    .await;
    let third_cookie = response_cookie(&third_cart);
    let (_, abandoned_order_id) =
        create_stripe_order(&router, variant_id, "abandoned", Some(third_cookie.clone())).await;
    assert_eq!(
        request(
            &router,
            "POST",
            &format!("/api/orders/{abandoned_order_id}/payment"),
            Some(&third_cookie),
            None,
            None,
        )
        .await
        .status(),
        StatusCode::OK
    );
    sqlx::query(
        "UPDATE payment_attempts SET expires_at = now() - interval '2 hours' WHERE order_payment_id = (SELECT id FROM order_payments WHERE order_id = $1)",
    )
    .bind(Uuid::parse_str(&abandoned_order_id).unwrap())
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        knitprint_api::payments::cleanup_abandoned(&pool, 100)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        knitprint_api::payments::cleanup_abandoned(&pool, 100)
            .await
            .unwrap(),
        0
    );
    let abandoned_order = request(
        &router,
        "GET",
        &format!("/api/orders/{abandoned_order_id}"),
        Some(&third_cookie),
        None,
        None,
    )
    .await;
    let abandoned_body = response_json(abandoned_order).await;
    assert_eq!(abandoned_body["order_status"], "cancelled");
    assert_eq!(
        abandoned_body["payment"]["attempts"][0]["failure_code"],
        "checkout_abandoned"
    );
    let quantities: (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, committed_quantity FROM inventory_items WHERE variant_id = $1",
    )
    .bind(variant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quantities, (5, 0, 2));

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

async fn create_stripe_order(
    router: &axum::Router,
    variant_id: Uuid,
    suffix: &str,
    cookie: Option<String>,
) -> (String, String) {
    let cookie = match cookie {
        Some(cookie) => cookie,
        None => {
            let cart = request(router, "GET", "/api/cart", None, None, None).await;
            response_cookie(&cart)
        }
    };
    let added = request(
        router,
        "POST",
        "/api/cart/items",
        Some(&cookie),
        Some(json!({ "variant_id": variant_id, "quantity": 2 })),
        Some(&format!("stripe-cart-add-{suffix}-0001")),
    )
    .await;
    assert_eq!(added.status(), StatusCode::CREATED);
    let delivered = request(
        router,
        "POST",
        "/api/cart/delivery",
        Some(&cookie),
        Some(delivery_fixture()),
        Some(&format!("stripe-delivery-{suffix}-0001")),
    )
    .await;
    assert_eq!(delivered.status(), StatusCode::OK);
    let created = request(
        router,
        "POST",
        "/api/orders",
        Some(&cookie),
        Some(json!({ "payment_method": "stripe" })),
        Some(&format!("stripe-checkout-{suffix}-0001")),
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = response_json(created).await;
    (cookie, body["id"].as_str().unwrap().to_owned())
}

fn stripe_event(
    event_id: &str,
    event_type: &str,
    session_id: &str,
    order_id: &str,
    payment_status: &str,
) -> String {
    json!({
        "id": event_id,
        "type": event_type,
        "data": {
            "object": {
                "id": session_id,
                "object": "checkout.session",
                "payment_status": payment_status,
                "metadata": { "order_id": order_id }
            }
        }
    })
    .to_string()
}

async fn webhook_request(
    router: &axum::Router,
    secret: &str,
    payload: &str,
) -> axum::response::Response {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{timestamp}.{payload}").as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/payments/stripe/webhook")
                .header("stripe-signature", format!("t={timestamp},v1={signature}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap()
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
