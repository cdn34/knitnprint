use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use knitnprint_api::{AppState, app};
use serde_json::Value;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{env, str::FromStr};
use tower::ServiceExt;
use uuid::Uuid;

async fn create(pool: &PgPool, slug: &str) -> (Uuid, String, String) {
    let id = Uuid::new_v4();
    let (sku,slug)=sqlx::query_as::<_,(String,String)>("INSERT INTO products(id,title,slug,status,published_at) VALUES($1,'Linen shirt',$2,'active',now()) RETURNING sku,slug").bind(id).bind(slug).fetch_one(pool).await.unwrap();
    (id, sku, slug)
}
async fn get(router: &axum::Router, slug: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/products/{slug}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn sku_slugs_preserve_identity_across_migration_renames_deletion_and_replacement() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("slug_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    let options = PgConnectOptions::from_str(&url)
        .unwrap()
        .options([("search_path", format!("{schema},public"))]);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .unwrap();
    // Install the previous release and migrate an actual pre-existing URL.
    let migrator = sqlx::migrate!("../migrations");
    for migration in migrator.iter().filter(|m| m.version < 60) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let (first, sku, bare) = create(&pool, "linen-shirt").await;
    assert_eq!(bare, "linen-shirt");
    sqlx::raw_sql(include_str!("../../migrations/0060_product_sku_slugs.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let canonical: String = sqlx::query_scalar("SELECT slug FROM products WHERE id=$1")
        .bind(first)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(canonical, format!("linen-shirt-{}", sku.to_lowercase()));
    let router = app(AppState {
        database: Some(pool.clone()),
        ..AppState::default()
    });
    let (status, old) = get(&router, &bare).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(old["slug"], canonical);
    let (second, second_sku, second_url) = create(&pool, "linen-shirt").await;
    assert_ne!(sku, second_sku);
    assert_ne!(canonical, second_url);
    assert_eq!(get(&router, &bare).await.1["id"], first.to_string());
    assert_eq!(get(&router, &second_url).await.1["id"], second.to_string());
    let renamed: String = sqlx::query_scalar(
        "UPDATE products SET slug='summer-linen-shirt' WHERE id=$1 RETURNING slug",
    )
    .bind(first)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        renamed,
        format!("summer-linen-shirt-{}", sku.to_lowercase())
    );
    assert_eq!(get(&router, &canonical).await.1["slug"], renamed);
    let unchanged: String =
        sqlx::query_scalar("UPDATE products SET slug=$2 WHERE id=$1 RETURNING slug")
            .bind(first)
            .bind(&renamed)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(unchanged, renamed);
    let (_, _, collision) = create(&pool, &canonical).await;
    assert_ne!(collision, canonical);
    assert_eq!(get(&router, &canonical).await.1["id"], first.to_string());
    sqlx::query("DELETE FROM products WHERE id=$1")
        .bind(first)
        .execute(&pool)
        .await
        .unwrap();
    let (replacement, replacement_sku, replacement_url) = create(&pool, "linen-shirt").await;
    assert_ne!(replacement_sku, sku);
    assert_ne!(replacement_url, canonical);
    for old in [&bare, &canonical, &renamed] {
        assert_eq!(get(&router, old).await.0, StatusCode::NOT_FOUND);
    }
    assert_eq!(
        get(&router, &replacement_url).await.1["id"],
        replacement.to_string()
    );
    assert!(
        sqlx::query("UPDATE product_slug_history SET product_id=$2 WHERE slug=$1")
            .bind(&bare)
            .bind(replacement)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM product_slug_history WHERE slug=$1")
            .bind(&bare)
            .execute(&pool)
            .await
            .is_err()
    );
    let (_, long_sku, long_url) = create(&pool, &"a".repeat(200)).await;
    assert_eq!(long_url.len(), 200);
    assert!(long_url.ends_with(&long_sku.to_lowercase()));
    let (a, b) = tokio::join!(create(&pool, "same-name"), create(&pool, "same-name"));
    assert_ne!(a.2, b.2);
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
