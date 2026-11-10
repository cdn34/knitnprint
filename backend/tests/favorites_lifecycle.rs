use std::{env, str::FromStr};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use knitnprint_api::{AppState, app};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn favorites_persist_are_private_and_only_expose_published_catalog_data() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .unwrap();
    let schema = format!("favorites_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&admin)
        .await
        .unwrap();
    let search_path = format!(r#"SET search_path TO "{schema}", public"#);
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query(&search_path).execute(connection).await?;
                Ok(())
            })
        })
        .connect_with(PgConnectOptions::from_str(&database_url).unwrap())
        .await
        .unwrap();
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let router = app(AppState {
        database: Some(pool.clone()),
        ..AppState::default()
    });
    let (customer, cookie) = create_customer(&pool, "favorite@example.test").await;
    let (other, other_cookie) = create_customer(&pool, "other-favorite@example.test").await;
    let product = create_product(&pool, "favorite-piece", "active").await;
    let draft = create_product(&pool, "private-piece", "draft").await;
    let path = format!("/api/account/favorites/{product}");

    for (method, uri) in [
        ("GET", "/api/account/favorites"),
        ("PUT", path.as_str()),
        ("DELETE", path.as_str()),
    ] {
        assert_eq!(
            request(&router, method, uri, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        body(request(&router, "GET", "/api/account/favorites", Some(&cookie)).await).await,
        json!([])
    );

    // Concurrent and repeated clicks are idempotent and preserve the original save date.
    let (first, duplicate) = tokio::join!(
        request(&router, "PUT", &path, Some(&cookie)),
        request(&router, "PUT", &path, Some(&cookie))
    );
    assert_eq!(first.status(), StatusCode::NO_CONTENT);
    assert_eq!(duplicate.status(), StatusCode::NO_CONTENT);
    let saved_at: String = sqlx::query_scalar("SELECT created_at::text FROM customer_favorites WHERE customer_id = $1 AND product_id = $2").bind(customer).bind(product).fetch_one(&pool).await.unwrap();
    assert_eq!(
        request(&router, "PUT", &path, Some(&cookie)).await.status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT created_at::text FROM customer_favorites WHERE customer_id = $1 AND product_id = $2").bind(customer).bind(product).fetch_one(&pool).await.unwrap(), saved_at);
    assert_eq!(
        request(
            &router,
            "PUT",
            &format!("/api/account/favorites/{draft}"),
            Some(&cookie)
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &router,
            "PUT",
            &format!("/api/account/favorites/{}", Uuid::new_v4()),
            Some(&cookie)
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    let response = request(&router, "GET", "/api/account/favorites", Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    let favorites = body(response).await;
    assert_eq!(favorites.as_array().unwrap().len(), 1);
    assert_eq!(favorites[0]["id"], product.to_string());
    assert_eq!(favorites[0]["search_keywords"], "");
    assert_eq!(favorites[0]["variants"][0]["available_quantity"], 100);
    assert_eq!(favorites[0]["variants"][0]["low_stock"], false);
    assert_eq!(favorites[0]["variants"][0]["price_minor"], 2000);
    assert_eq!(
        body(
            request(
                &router,
                "GET",
                "/api/account/favorites",
                Some(&other_cookie)
            )
            .await
        )
        .await,
        json!([])
    );

    // A different customer cannot remove this customer's favorite.
    assert_eq!(
        request(&router, "DELETE", &path, Some(&other_cookie))
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(count(&pool, customer).await, 1);
    assert_eq!(
        request(&router, "PUT", &path, Some(&other_cookie))
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&router, "DELETE", &path, Some(&cookie))
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&router, "DELETE", &path, Some(&cookie))
            .await
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(count(&pool, customer).await, 0);
    assert_eq!(count(&pool, other).await, 1);

    // Hidden products disappear; republishing retains the favorite with current prices.
    sqlx::query("UPDATE products SET status = 'archived' WHERE id = $1")
        .bind(product)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        body(
            request(
                &router,
                "GET",
                "/api/account/favorites",
                Some(&other_cookie)
            )
            .await
        )
        .await,
        json!([])
    );
    sqlx::query("UPDATE products SET status = 'active' WHERE id = $1")
        .bind(product)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE product_variants SET price_minor = 2500 WHERE product_id = $1")
        .bind(product)
        .execute(&pool)
        .await
        .unwrap();
    let restored = body(
        request(
            &router,
            "GET",
            "/api/account/favorites",
            Some(&other_cookie),
        )
        .await,
    )
    .await;
    assert_eq!(restored[0]["variants"][0]["price_minor"], 2500);

    assert_eq!(
        request(&router, "PUT", &path, Some(&cookie)).await.status(),
        StatusCode::NO_CONTENT
    );
    sqlx::query("DELETE FROM customer_accounts WHERE customer_id = $1")
        .bind(customer)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(count(&pool, customer).await, 0);
    assert_eq!(
        request(&router, "GET", "/api/account/favorites", Some(&cookie))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(product)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(count(&pool, other).await, 0);

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

async fn create_customer(pool: &PgPool, email: &str) -> (Uuid, String) {
    let id = Uuid::now_v7();
    let token = format!("favorites-session-{id}");
    sqlx::query("INSERT INTO customers (id, customer_type, email, first_name, last_name) VALUES ($1, 'registered', $2, 'Ana', 'Silva')").bind(id).bind(email).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO customer_accounts (customer_id, password_hash, email_verified_at) VALUES ($1, 'test-session-only', now())").bind(id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO customer_sessions (id, customer_id, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 day')").bind(Uuid::now_v7()).bind(id).bind(Sha256::digest(token.as_bytes()).as_slice()).execute(pool).await.unwrap();
    (id, format!("knitnprint_customer={token}"))
}

async fn create_product(pool: &PgPool, slug: &str, status: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO products (id, title, slug, status, published_at, search_keywords) VALUES ($1, 'Favorite piece', $2, $3, now(), 'private staff keywords')").bind(id).bind(slug).bind(status).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO product_variants (id, product_id, title, sku, price_minor, currency) VALUES ($1, $2, 'Default', $3, 2000, 'EUR')").bind(Uuid::now_v7()).bind(id).bind(slug).execute(pool).await.unwrap();
    id
}

async fn count(pool: &PgPool, customer: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM customer_favorites WHERE customer_id = $1")
        .bind(customer)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn request(
    router: &axum::Router,
    method: &str,
    path: &str,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}
