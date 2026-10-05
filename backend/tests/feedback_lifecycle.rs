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

    // The homepage stays bounded while its aggregates include every approved review.
    sqlx::query("INSERT INTO product_feedback (id, display_name, rating, comment, status, created_at) SELECT md5('feedback-page-' || n)::uuid, 'Page tester', 5, 'A lovely store experience.', 'approved', now() FROM generate_series(1, 320) AS n")
        .execute(&pool).await.unwrap();
    let homepage = response_json(request(&router, "GET", "/api/feedback", None).await).await;
    assert_eq!(homepage["total_reviews"], 321);
    assert_eq!(homepage["reviews"].as_array().unwrap().len(), 20);
    let first_page = response_json(
        staff_request(
            &router,
            "GET",
            "/api/admin/feedback?status=approved&limit=100&offset=0",
            cookie,
            None,
        )
        .await,
    )
    .await;
    let second_page = response_json(
        staff_request(
            &router,
            "GET",
            "/api/admin/feedback?status=approved&limit=100&offset=100",
            cookie,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(first_page.as_array().unwrap().len(), 100);
    assert_eq!(second_page.as_array().unwrap().len(), 100);
    assert!(first_page.as_array().unwrap().iter().all(|item| {
        !second_page
            .as_array()
            .unwrap()
            .iter()
            .any(|next| next["id"] == item["id"])
    }));
    let last_page = response_json(
        staff_request(
            &router,
            "GET",
            "/api/admin/feedback?status=approved&limit=100&offset=300",
            cookie,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(last_page.as_array().unwrap().len(), 22);
    for limit in [0, 101] {
        assert_eq!(
            staff_request(
                &router,
                "GET",
                &format!("/api/admin/feedback?limit={limit}"),
                cookie,
                None
            )
            .await
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    // Both submissions above used the same IP; changing endpoints cannot bypass its limit.
    let blocked = request(
        &router,
        "POST",
        "/api/feedback",
        Some(json!({
            "display_name": "Blocked", "rating": 5, "comment": "This must never be inserted."
        })),
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after: u64 = blocked.headers()[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=3600).contains(&retry_after));
    let inserted: i64 =
        sqlx::query_scalar("SELECT count(*) FROM product_feedback WHERE display_name='Blocked'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(inserted, 0);

    // Concurrent callers on separate connections still get exactly two inserts per IP.
    sqlx::query("DELETE FROM auth_login_rate_limits WHERE auth_scope='feedback'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE products SET status='active' WHERE id=$1")
        .bind(product_id)
        .execute(&pool)
        .await
        .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(9));
    let mut attempts = Vec::new();
    for n in 0..8 {
        let router = router.clone();
        let barrier = barrier.clone();
        attempts.push(tokio::spawn(async move {
            barrier.wait().await;
            let path = if n % 2 == 0 {
                "/api/feedback"
            } else {
                "/api/products/story-tee/feedback"
            };
            ip_request(&router, path, "203.0.113.10:4000").await
        }));
    }
    barrier.wait().await;
    let mut accepted = 0;
    for attempt in attempts {
        match attempt.await.unwrap().status() {
            StatusCode::ACCEPTED => accepted += 1,
            StatusCode::TOO_MANY_REQUESTS => {}
            status => panic!("unexpected submission status: {status}"),
        }
    }
    assert_eq!(accepted, 2);
    assert_eq!(
        ip_request(&router, "/api/feedback", "203.0.113.11:4000")
            .await
            .status(),
        StatusCode::ACCEPTED
    );

    // Rotating IPs cannot bypass the global budget or grow the limiter table indefinitely.
    sqlx::query("UPDATE auth_login_rate_limits SET event_count=19 WHERE auth_scope='feedback' AND dimension='global'").execute(&pool).await.unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(5));
    let mut attempts = Vec::new();
    for n in 12..16 {
        let router = router.clone();
        let barrier = barrier.clone();
        attempts.push(tokio::spawn(async move {
            barrier.wait().await;
            ip_request(&router, "/api/feedback", &format!("203.0.113.{n}:4000")).await
        }));
    }
    barrier.wait().await;
    let mut accepted = 0;
    for attempt in attempts {
        match attempt.await.unwrap().status() {
            StatusCode::ACCEPTED => accepted += 1,
            StatusCode::TOO_MANY_REQUESTS => {}
            status => panic!("unexpected global limit status: {status}"),
        }
    }
    assert_eq!(accepted, 1);
    let bucket_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth_login_rate_limits WHERE auth_scope='feedback'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        ip_request(&router, "/api/feedback", "203.0.113.99:4000")
            .await
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth_login_rate_limits WHERE auth_scope='feedback'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bucket_count, after);

    let current = response_json(
        staff_request(&router, "GET", "/api/admin/feedback/settings", cookie, None).await,
    )
    .await;
    assert_eq!(
        current,
        json!({"daily_submission_limit":20,"hourly_ip_limit":2})
    );
    assert_eq!(
        request(
            &router,
            "PUT",
            "/api/admin/feedback/settings",
            Some(json!({"daily_submission_limit":30,"hourly_ip_limit":3}))
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        staff_request(
            &router,
            "PUT",
            "/api/admin/feedback/settings",
            cookie,
            Some(json!({"daily_submission_limit":0,"hourly_ip_limit":0}))
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let saved = staff_request(
        &router,
        "PUT",
        "/api/admin/feedback/settings",
        cookie,
        Some(json!({"daily_submission_limit":30,"hourly_ip_limit":3})),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    assert_eq!(
        response_json(saved).await,
        json!({"daily_submission_limit":30,"hourly_ip_limit":3})
    );
    assert_eq!(
        response_json(
            staff_request(&router, "GET", "/api/admin/feedback/settings", cookie, None).await
        )
        .await,
        json!({"daily_submission_limit":30,"hourly_ip_limit":3})
    );
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action='feedback.settings_update'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audited, 1);

    // A staff member with read access can inspect limits but cannot modify them.
    let reader_id = Uuid::now_v7();
    sqlx::query("INSERT INTO staff_users (id,email,display_name,password_hash,role) VALUES ($1,'reader@feedback.test','Feedback reader',$2,'staff')")
        .bind(reader_id).bind(hash_password("integration-test-passphrase").unwrap()).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO staff_capabilities (staff_user_id,capability_name) VALUES ($1,'catalog.read')",
    )
    .bind(reader_id)
    .execute(&pool)
    .await
    .unwrap();
    let reader_login = request(
        &router,
        "POST",
        "/api/admin/auth/login",
        Some(json!({"email":"reader@feedback.test","password":"integration-test-passphrase"})),
    )
    .await;
    assert_eq!(reader_login.status(), StatusCode::OK);
    let reader_cookie = reader_login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    assert_eq!(
        staff_request(
            &router,
            "GET",
            "/api/admin/feedback/settings",
            reader_cookie,
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        staff_request(
            &router,
            "PUT",
            "/api/admin/feedback/settings",
            reader_cookie,
            Some(json!({"daily_submission_limit":100,"hourly_ip_limit":100}))
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );

    // Raising the saved limits immediately permits another submission in existing windows.
    assert_eq!(
        ip_request(&router, "/api/feedback", "203.0.113.10:4000")
            .await
            .status(),
        StatusCode::ACCEPTED
    );
    let saved = staff_request(
        &router,
        "PUT",
        "/api/admin/feedback/settings",
        cookie,
        Some(json!({"daily_submission_limit":30,"hourly_ip_limit":2})),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    // Lowering the limit immediately blocks an IP which has already exceeded it.
    assert_eq!(
        ip_request(&router, "/api/feedback", "203.0.113.10:4000")
            .await
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    // Only the IP window expires after an hour; the global daily count remains.
    sqlx::query("UPDATE auth_login_rate_limits SET window_started_at=now()-interval '61 minutes' WHERE auth_scope='feedback' AND dimension='ip'").execute(&pool).await.unwrap();
    assert_eq!(
        ip_request(&router, "/api/feedback", "203.0.113.10:4000")
            .await
            .status(),
        StatusCode::ACCEPTED
    );

    // Expired windows allow submissions again.
    sqlx::query("UPDATE auth_login_rate_limits SET window_started_at=now()-interval '2 days', locked_until=now()-interval '1 day' WHERE auth_scope='feedback'").execute(&pool).await.unwrap();
    assert_eq!(
        ip_request(&router, "/api/feedback", "203.0.113.10:4000")
            .await
            .status(),
        StatusCode::ACCEPTED
    );

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

async fn ip_request(router: &axum::Router, path: &str, peer: &str) -> axum::response::Response {
    router.clone().oneshot(Request::builder()
        .method("POST").uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .extension(axum::extract::ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap()))
        .body(Body::from(json!({"display_name":"Concurrency tester", "rating":5, "comment":"A beautiful store experience."}).to_string())).unwrap()
    ).await.unwrap()
}
