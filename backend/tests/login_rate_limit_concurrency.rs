use std::{
    env,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use knitprint_api::{
    AppState, app,
    login_rate_limit::{LoginLimitError, consume_account_action},
};
use serde_json::json;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::Barrier;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn concurrent_account_and_ip_limits_are_exact_and_scope_safe() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("login_limit_test_{}", Uuid::new_v4().simple());
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
    let router = app(AppState {
        database: Some(pool.clone()),
        ..AppState::default()
    });

    let barrier = Arc::new(Barrier::new(13));
    let mut attempts = Vec::new();
    for _ in 0..12 {
        let router = router.clone();
        let barrier = barrier.clone();
        attempts.push(tokio::spawn(async move {
            barrier.wait().await;
            login(
                &router,
                "contended@example.test",
                "short",
                "203.0.113.10:4000",
            )
            .await
        }));
    }
    barrier.wait().await;
    let mut unauthorized = 0;
    let mut limited = 0;
    for attempt in attempts {
        let response = attempt.await.expect("login task should complete");
        match response.status() {
            StatusCode::UNAUTHORIZED => unauthorized += 1,
            StatusCode::TOO_MANY_REQUESTS => {
                limited += 1;
                assert_eq!(
                    response.headers().get(header::RETRY_AFTER),
                    Some(&header::HeaderValue::from_static("900"))
                );
            }
            status => panic!("unexpected concurrent login status: {status}"),
        }
    }
    assert_eq!(unauthorized, 5);
    assert_eq!(limited, 7);
    let account_bucket: (i32, bool) = sqlx::query_as(
        r#"
        SELECT event_count, locked_until > now()
        FROM auth_login_rate_limits
        WHERE auth_scope = 'customer' AND dimension = 'account'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("account limiter should be stored");
    assert_eq!(account_bucket, (5, true));

    for sequence in 0..60 {
        let response = login(
            &router,
            &format!("ip-volume-{sequence}@example.test"),
            "short",
            "203.0.113.11:4000",
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let ip_limited = login(
        &router,
        "ip-volume-final@example.test",
        "short",
        "203.0.113.11:4000",
    )
    .await;
    assert_eq!(ip_limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        ip_limited.headers().get(header::RETRY_AFTER),
        Some(&header::HeaderValue::from_static("300"))
    );

    let invalid_hashes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth_login_rate_limits WHERE octet_length(key_hash) <> 32",
    )
    .fetch_one(&pool)
    .await
    .expect("rate-limit hashes should be inspectable");
    assert_eq!(invalid_hashes, 0);
    let raw_identifier_columns: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'auth_login_rate_limits'
          AND column_name IN ('email', 'ip_address')
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("rate-limit schema should be inspectable");
    assert_eq!(raw_identifier_columns, 0);

    let action_ip = IpAddr::from_str("203.0.113.12").unwrap();
    for _ in 0..5 {
        consume_account_action(&pool, "password_reset", "target@example.test", action_ip)
            .await
            .expect("the bounded account-action allowance should pass");
    }
    assert!(matches!(
        consume_account_action(&pool, "password_reset", "target@example.test", action_ip,).await,
        Err(LoginLimitError::Limited(3600))
    ));

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .expect("test schema should be removed");
    admin.close().await;
}

async fn login(
    router: &axum::Router,
    email: &str,
    password: &str,
    peer: &str,
) -> axum::response::Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/account/login")
                .header(header::CONTENT_TYPE, "application/json")
                .extension(ConnectInfo(SocketAddr::from_str(peer).unwrap()))
                .body(Body::from(
                    json!({ "email": email, "password": password }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("login request should complete")
}

async fn isolated_pool(database_url: &str, schema: &str) -> PgPool {
    let options = PgConnectOptions::from_str(database_url).expect("DATABASE_URL should be valid");
    let search_path = format!(r#"SET search_path TO "{schema}", public"#);
    PgPoolOptions::new()
        .max_connections(16)
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
