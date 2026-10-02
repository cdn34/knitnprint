use std::{env, str::FromStr};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use knitnprint_api::{AppState, app, auth::hash_password};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn approved_product_and_storefront_feedback_share_the_homepage() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("feedback_test_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .unwrap();
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&admin)
        .await
        .unwrap();
    let pool = isolated_pool(&database_url, &schema).await;
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();

    let product_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO products (id,title,slug,description,status,published_at) VALUES ($1,'Story Tee','story-tee','A test piece','active',now())",
    )
    .bind(product_id)
    .execute(&pool)
    .await
    .unwrap();
    let router = app(AppState {
        database: Some(pool.clone()),
        ..AppState::default()
    });

    let product = request(
        &router,
        "POST",
        "/api/products/story-tee/feedback",
        Some(json!({
            "display_name": "Alex", "rating": 5, "comment": "A beautiful piece made with care."
        })),
    )
    .await;
    assert_eq!(product.status(), StatusCode::ACCEPTED);
    let product_id_feedback = response_json(product).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let shop = request(&router, "POST", "/api/feedback", Some(json!({
        "display_name": "Jamie", "rating": 4, "comment": "Thoughtful service and careful packaging."
    }))).await;
    assert_eq!(shop.status(), StatusCode::ACCEPTED);
    let shop_id_feedback = response_json(shop).await["id"].as_str().unwrap().to_owned();

    let pending = response_json(request(&router, "GET", "/api/feedback", None).await).await;
    assert_eq!(pending["total_reviews"], 0);
    assert_eq!(pending["reviews"], json!([]));

    let owner_id = Uuid::now_v7();
    sqlx::query("INSERT INTO staff_users (id,email,display_name,password_hash,role) VALUES ($1,'owner@feedback.test','Feedback owner',$2,'owner')")
        .bind(owner_id)
        .bind(hash_password("integration-test-passphrase").unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let login = request(
        &router,
        "POST",
        "/api/admin/auth/login",
        Some(json!({
            "email": "owner@feedback.test", "password": "integration-test-passphrase"
        })),
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    let queue = staff_request(
        &router,
        "GET",
        "/api/admin/feedback?status=pending",
        cookie,
        None,
    )
    .await;
    assert_eq!(queue.status(), StatusCode::OK);
    assert_eq!(response_json(queue).await.as_array().unwrap().len(), 2);
    for id in [&product_id_feedback, &shop_id_feedback] {
        let approved = staff_request(
            &router,
            "PUT",
            &format!("/api/admin/feedback/{id}"),
            cookie,
            Some(json!({"status":"approved"})),
        )
        .await;
        assert_eq!(approved.status(), StatusCode::OK);
    }

    let homepage = response_json(request(&router, "GET", "/api/feedback", None).await).await;
    assert_eq!(homepage["total_reviews"], 2);
    assert_eq!(homepage["average_rating"], 4.5);
    assert_eq!(homepage["reviews"].as_array().unwrap().len(), 2);
    assert!(homepage["reviews"].as_array().unwrap().iter().any(|item| {
        item["product_slug"] == "story-tee" && item["product_title"] == "Story Tee"
    }));
    assert!(
        homepage["reviews"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item["product_slug"].is_null() && item["display_name"] == "Jamie" })
    );

    let product_page =
        response_json(request(&router, "GET", "/api/products/story-tee/feedback", None).await)
            .await;
    assert_eq!(product_page["total_reviews"], 1);
    assert_eq!(product_page["reviews"][0]["display_name"], "Alex");

    sqlx::query("UPDATE products SET status='archived' WHERE id=$1")
        .bind(product_id)
        .execute(&pool)
        .await
        .unwrap();
    let after_archive = response_json(request(&router, "GET", "/api/feedback", None).await).await;
    assert_eq!(after_archive["total_reviews"], 1);
    assert_eq!(after_archive["reviews"][0]["display_name"], "Jamie");

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
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
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(path);
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

async fn staff_request(
    router: &axum::Router,
    method: &str,
    path: &str,
    cookie: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, cookie);
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

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
