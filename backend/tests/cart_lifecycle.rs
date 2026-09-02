use std::{env, str::FromStr};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use knitprint_api::{AppState, app, carts::cleanup_expired};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn cart_prices_stock_delivery_and_retries_are_server_controlled() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("cart_test_{}", Uuid::new_v4().simple());
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
    let variant_id = insert_product(&pool).await;
    let router = app(AppState {
        database: Some(pool.clone()),
        ..AppState::default()
    });

    let empty = request(&router, "GET", "/api/cart", None, None, None).await;
    assert_eq!(empty.status(), StatusCode::OK);
    assert_eq!(empty.headers()[header::CACHE_CONTROL], "no-store, private");
    let cart_cookie = response_cookie(&empty);
    assert!(cart_cookie.starts_with("knitprint_cart="));
    assert_eq!(response_json(empty).await["items"], json!([]));

    let added = request(
        &router,
        "POST",
        "/api/cart/items",
        Some(&cart_cookie),
        Some(json!({ "variant_id": variant_id, "quantity": 2 })),
        Some("cart-add-fixture-0001"),
    )
    .await;
    assert_eq!(added.status(), StatusCode::CREATED);
    let added_body = response_json(added).await;
    assert_eq!(added_body["item_count"], 2);
    assert_eq!(added_body["subtotal_minor"], 5000);
    assert_eq!(added_body["items"][0]["unit_price_minor"], 2500);
    let line_id = added_body["items"][0]["id"].as_str().unwrap();

    let replay = request(
        &router,
        "POST",
        "/api/cart/items",
        Some(&cart_cookie),
        Some(json!({ "variant_id": variant_id, "quantity": 2 })),
        Some("cart-add-fixture-0001"),
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(response_json(replay).await["item_count"], 2);
    let conflict = request(
        &router,
        "POST",
        "/api/cart/items",
        Some(&cart_cookie),
        Some(json!({ "variant_id": variant_id, "quantity": 1 })),
        Some("cart-add-fixture-0001"),
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    sqlx::query("UPDATE product_variants SET price_minor = 2700 WHERE id = $1")
        .bind(variant_id)
        .execute(&pool)
        .await
        .unwrap();
    let repriced = request(&router, "GET", "/api/cart", Some(&cart_cookie), None, None).await;
    let repriced_body = response_json(repriced).await;
    assert_eq!(repriced_body["subtotal_minor"], 5400);
    assert_eq!(repriced_body["issues"][0]["code"], "price_changed");

    sqlx::query("UPDATE inventory_items SET available_quantity = 1 WHERE variant_id = $1")
        .bind(variant_id)
        .execute(&pool)
        .await
        .unwrap();
    let short = request(&router, "GET", "/api/cart", Some(&cart_cookie), None, None).await;
    let short_body = response_json(short).await;
    assert_eq!(short_body["items"][0]["available"], true);
    assert_eq!(short_body["items"][0]["available_quantity"], 100);
    assert_eq!(short_body["issues"], json!([]));
    assert_eq!(short_body["checkout_ready"], false);

    let updated = request(
        &router,
        "PATCH",
        &format!("/api/cart/items/{line_id}"),
        Some(&cart_cookie),
        Some(json!({ "quantity": 100 })),
        Some("cart-update-fixture-01"),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(response_json(updated).await["item_count"], 100);

    let delivery = request(
        &router,
        "POST",
        "/api/cart/delivery",
        Some(&cart_cookie),
        Some(delivery_fixture()),
        Some("cart-delivery-fixture-01"),
    )
    .await;
    assert_eq!(delivery.status(), StatusCode::OK);
    let delivery_body = response_json(delivery).await;
    assert_eq!(delivery_body["delivery"]["email"], "ada@example.com");
    assert_eq!(delivery_body["delivery"]["address"]["country_code"], "PT");
    assert_eq!(delivery_body["checkout_ready"], true);
    let delivery_replay = request(
        &router,
        "POST",
        "/api/cart/delivery",
        Some(&cart_cookie),
        Some(delivery_fixture()),
        Some("cart-delivery-fixture-01"),
    )
    .await;
    assert_eq!(delivery_replay.status(), StatusCode::OK);
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM customers), (SELECT count(*) FROM customer_addresses)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        counts,
        (1, 1),
        "delivery retries must not duplicate private data"
    );

    sqlx::query("UPDATE products SET status = 'archived' WHERE slug = 'cart-vase'")
        .execute(&pool)
        .await
        .unwrap();
    let archived = request(&router, "GET", "/api/cart", Some(&cart_cookie), None, None).await;
    let archived_body = response_json(archived).await;
    assert_eq!(archived_body["issues"][0]["code"], "product_unavailable");
    assert_eq!(archived_body["checkout_ready"], false);

    let removed = request(
        &router,
        "DELETE",
        &format!("/api/cart/items/{line_id}"),
        Some(&cart_cookie),
        None,
        Some("cart-remove-fixture-01"),
    )
    .await;
    assert_eq!(removed.status(), StatusCode::OK);
    let removed_body = response_json(removed).await;
    assert_eq!(removed_body["items"], json!([]));
    assert_eq!(removed_body["currency"], Value::Null);

    let expired_cart_id = Uuid::parse_str(removed_body["id"].as_str().unwrap()).unwrap();
    sqlx::query(
        "UPDATE carts SET created_at = now() - interval '31 days', expires_at = now() - interval '1 minute' WHERE id = $1",
    )
        .bind(expired_cart_id)
        .execute(&pool)
        .await
        .unwrap();
    let replacement = request(&router, "GET", "/api/cart", Some(&cart_cookie), None, None).await;
    assert_eq!(replacement.status(), StatusCode::OK);
    assert!(replacement.headers().contains_key(header::SET_COOKIE));
    let replacement_body = response_json(replacement).await;
    assert_ne!(replacement_body["id"], removed_body["id"]);
    assert_eq!(cleanup_expired(&pool, 100).await.unwrap(), 1);
    let expired_remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM carts WHERE id = $1")
        .bind(expired_cart_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(expired_remaining, 0);

    let token = cart_cookie.split_once('=').unwrap().1;
    let raw_token_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM carts WHERE token_hash = convert_to($1, 'UTF8')")
            .bind(token)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(raw_token_count, 0, "raw cart tokens must never be stored");

    let registered = request(
        &router,
        "POST",
        "/api/account/register",
        None,
        Some(json!({
            "email": "registered-cart@example.com",
            "password": "registered-cart-passphrase",
            "first_name": "Registered",
            "last_name": "Cart"
        })),
        None,
    )
    .await;
    assert_eq!(registered.status(), StatusCode::CREATED);
    let account_cookie = response_cookie(&registered);
    let registered_body = response_json(registered).await;
    let registered_customer_id = registered_body["id"].as_str().unwrap();
    let account_cart = request(
        &router,
        "GET",
        "/api/cart",
        Some(&account_cookie),
        None,
        None,
    )
    .await;
    let account_cart_cookie = response_cookie(&account_cart);
    let combined_cookies = format!("{account_cookie}; {account_cart_cookie}");
    let account_delivery = request(
        &router,
        "POST",
        "/api/cart/delivery",
        Some(&combined_cookies),
        Some(delivery_fixture()),
        Some("account-cart-delivery-01"),
    )
    .await;
    assert_eq!(account_delivery.status(), StatusCode::OK);
    assert_eq!(
        response_json(account_delivery).await["delivery"]["customer_id"],
        registered_customer_id
    );
    let final_customer_count: i64 = sqlx::query_scalar("SELECT count(*) FROM customers")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        final_customer_count, 2,
        "an authenticated cart must reuse the registered customer"
    );

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .expect("test schema should be removed");
    admin.close().await;
}

async fn insert_product(pool: &PgPool) -> Uuid {
    let product_id = Uuid::now_v7();
    let variant_id = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO products (id, title, slug, description, status, published_at)
        VALUES ($1, 'Cart Vase', 'cart-vase', 'A cart fixture', 'active', now())
        "#,
    )
    .bind(product_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO product_variants (id, product_id, title, sku, price_minor, currency)
        VALUES ($1, $2, 'Sand', 'CART-SAND', 2500, 'EUR')
        "#,
    )
    .bind(variant_id)
    .bind(product_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("UPDATE inventory_items SET available_quantity = 8 WHERE variant_id = $1")
        .bind(variant_id)
        .execute(pool)
        .await
        .unwrap();
    variant_id
}

fn delivery_fixture() -> Value {
    json!({
        "email": " ADA@Example.COM ",
        "first_name": "Ada",
        "last_name": "Lovelace",
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
        .expect("isolated test pool should connect")
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
    if let Some(idempotency_key) = idempotency_key {
        builder = builder.header("idempotency-key", idempotency_key);
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
