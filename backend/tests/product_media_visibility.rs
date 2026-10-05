use std::{
    collections::HashMap,
    env,
    io::Cursor,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use image::{DynamicImage, ImageFormat, RgbaImage};
use knitnprint_api::{
    AppState, app,
    auth::hash_password,
    object_storage::{
        ObjectMetadata, ObjectStorage, StorageBackend, StorageError, StorageProvider, StoredObject,
    },
};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Default)]
struct MemoryStorage(Mutex<HashMap<String, (String, Vec<u8>)>>);

#[async_trait]
impl StorageBackend for MemoryStorage {
    fn provider(&self) -> StorageProvider {
        StorageProvider::SeaweedFs
    }
    fn bucket(&self) -> &str {
        "test-media"
    }
    async fn presign_upload(
        &self,
        key: &str,
        _: &str,
        _: i64,
        _: Duration,
    ) -> Result<String, StorageError> {
        Ok(format!("http://storage.test/{key}"))
    }
    async fn presign_download(&self, key: &str, _: Duration) -> Result<String, StorageError> {
        Ok(format!("http://storage.test/{key}"))
    }
    async fn head(&self, key: &str) -> Result<ObjectMetadata, StorageError> {
        let objects = self.0.lock().unwrap();
        let (content_type, bytes) = objects.get(key).ok_or(StorageError::Head)?;
        Ok(ObjectMetadata {
            content_length: Some(bytes.len() as i64),
            content_type: Some(content_type.clone()),
        })
    }
    async fn get(&self, key: &str) -> Result<StoredObject, StorageError> {
        let objects = self.0.lock().unwrap();
        let (content_type, bytes) = objects.get(key).ok_or(StorageError::Read)?;
        Ok(StoredObject {
            bytes: bytes.clone(),
            content_type: Some(content_type.clone()),
            etag: None,
        })
    }
    async fn put(&self, key: &str, content_type: &str, bytes: Vec<u8>) -> Result<(), StorageError> {
        self.0
            .lock()
            .unwrap()
            .insert(key.into(), (content_type.into(), bytes));
        Ok(())
    }
    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.0.lock().unwrap().remove(key);
        Ok(())
    }
}

#[tokio::test]
async fn uploaded_draft_images_are_visible_only_to_catalog_staff_until_published() {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("product_media_test_{}", Uuid::new_v4().simple());
    let admin = PgPool::connect(&database_url).await.unwrap();
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
    let storage = ObjectStorage::from_backend(Arc::new(MemoryStorage::default()));
    let router = app(AppState {
        database: Some(pool.clone()),
        media_storage: Some(storage.clone()),
        ..AppState::default()
    });
    let owner_cookie = staff_cookie(&pool, &router, "owner@media.test", "owner", &[]).await;
    let reader_cookie = staff_cookie(
        &pool,
        &router,
        "reader@media.test",
        "staff",
        &["catalog.read"],
    )
    .await;
    let other_cookie = staff_cookie(
        &pool,
        &router,
        "other@media.test",
        "staff",
        &["orders.read"],
    )
    .await;
    let created = request(&router, "POST", "/api/admin/products", Some(&owner_cookie), Some(json!({
        "title":"Draft image product", "slug":"draft-image-product", "variants":[{"title":"Default", "sku":"DRAFT-IMAGE-TEST", "price_minor":1000, "currency":"EUR", "available_quantity":5}]
    }))).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = response_json(created).await;
    assert_eq!(created["status"], "draft");
    let product_id = created["id"].as_str().unwrap();
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(RgbaImage::new(2, 2))
        .write_to(&mut png, ImageFormat::Png)
        .unwrap();
    let png = png.into_inner();
    let initiated = request(
        &router,
        "POST",
        "/api/admin/media/uploads",
        Some(&owner_cookie),
        Some(json!({"filename":"test.png", "content_type":"image/png", "byte_size":png.len()})),
    )
    .await;
    assert_eq!(initiated.status(), StatusCode::CREATED);
    let initiated = response_json(initiated).await;
    let media_id = initiated["id"].as_str().unwrap();
    let key = format!("uploads-quarantine/{media_id}/original.png");
    storage.put(&key, "image/png", png).await.unwrap();
    let completed = request(
        &router,
        "POST",
        &format!("/api/admin/media/uploads/{media_id}/complete"),
        Some(&owner_cookie),
        Some(json!({"product_id":product_id, "alt_text":"Draft product image"})),
    )
    .await;
    assert_eq!(completed.status(), StatusCode::OK);

    // Both admin list and detail must return private URLs for every variant.
    let admin_detail = response_json(
        request(
            &router,
            "GET",
            &format!("/api/admin/products/{product_id}"),
            Some(&reader_cookie),
            None,
        )
        .await,
    )
    .await;
    let admin_list = response_json(
        request(
            &router,
            "GET",
            "/api/admin/products",
            Some(&reader_cookie),
            None,
        )
        .await,
    )
    .await;
    for product in [&admin_detail, &admin_list[0]] {
        for (field, variant) in [
            ("url", "detail"),
            ("thumbnail_url", "thumbnail"),
            ("card_url", "card"),
            ("detail_url", "detail"),
        ] {
            assert_eq!(
                product["media"][0][field],
                format!("/api/admin/product-media/{media_id}/{variant}")
            );
        }
    }
    for variant in ["thumbnail", "card", "detail"] {
        let path = format!("/api/admin/product-media/{media_id}/{variant}");
        let response = request(&router, "GET", &path, Some(&reader_cookie), None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/webp");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        image::load_from_memory_with_format(&bytes, ImageFormat::WebP).unwrap();
        assert_eq!(
            request(&router, "GET", &path, None, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            request(&router, "GET", &path, Some(&other_cookie), None)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            request(
                &router,
                "GET",
                &format!("/api/media/{media_id}/{variant}"),
                None,
                None
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        request(
            &router,
            "GET",
            &format!("/api/admin/product-media/{media_id}/original"),
            Some(&owner_cookie),
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &router,
            "GET",
            &format!("/api/admin/product-media/{}/detail", Uuid::new_v4()),
            Some(&owner_cookie),
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );

    // Publishing enables public images without changing the admin's private URLs.
    let published = request(
        &router,
        "POST",
        &format!("/api/admin/products/{product_id}/status"),
        Some(&owner_cookie),
        Some(json!({"status":"active"})),
    )
    .await;
    assert_eq!(published.status(), StatusCode::OK);
    let published = response_json(published).await;
    assert_eq!(
        published["media"][0]["detail_url"],
        format!("/api/admin/product-media/{media_id}/detail")
    );
    let public_detail = response_json(
        request(
            &router,
            "GET",
            "/api/products/draft-image-product",
            None,
            None,
        )
        .await,
    )
    .await;
    let public_list =
        response_json(request(&router, "GET", "/api/products", None, None).await).await;
    for product in [&public_detail, &public_list[0]] {
        assert_eq!(
            product["media"][0]["detail_url"],
            format!("/api/media/{media_id}/detail")
        );
    }
    assert_eq!(
        request(
            &router,
            "GET",
            &format!("/api/media/{media_id}/detail"),
            None,
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    let archived = request(
        &router,
        "POST",
        &format!("/api/admin/products/{product_id}/status"),
        Some(&owner_cookie),
        Some(json!({"status":"archived"})),
    )
    .await;
    assert_eq!(archived.status(), StatusCode::OK);
    assert_eq!(
        request(
            &router,
            "GET",
            &format!("/api/media/{media_id}/detail"),
            None,
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &router,
            "GET",
            &format!("/api/admin/product-media/{media_id}/detail"),
            Some(&reader_cookie),
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

async fn staff_cookie(
    pool: &PgPool,
    router: &axum::Router,
    email: &str,
    role: &str,
    capabilities: &[&str],
) -> String {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO staff_users (id,email,display_name,password_hash,role) VALUES ($1,$2,'Media tester',$3,$4)").bind(id).bind(email).bind(hash_password("integration-test-passphrase").unwrap()).bind(role).execute(pool).await.unwrap();
    for capability in capabilities {
        sqlx::query(
            "INSERT INTO staff_capabilities (staff_user_id,capability_name) VALUES ($1,$2)",
        )
        .bind(id)
        .bind(capability)
        .execute(pool)
        .await
        .unwrap();
    }
    let response = request(
        router,
        "POST",
        "/api/admin/auth/login",
        None,
        Some(json!({"email":email, "password":"integration-test-passphrase"})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
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
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    assert!(
        response.status().is_success(),
        "unexpected response: {}",
        response.status()
    );
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}
