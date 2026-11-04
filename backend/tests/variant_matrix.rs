use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use knitnprint_api::{
    AppState,
    auth::AuthenticatedStaff,
    variant_options::{SaveMatrix, save_matrix},
};
use serde_json::{Value, json};
use sha2::Digest;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{env, str::FromStr};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn matrix_preserves_legacy_identity_and_rejects_stale_or_invalid_changes() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("variants_{}", Uuid::new_v4().simple());
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
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let owner = Uuid::new_v4();
    let product = Uuid::new_v4();
    let legacy = Uuid::new_v4();
    sqlx::query("INSERT INTO staff_users(id,email,display_name,password_hash,role) VALUES($1,'owner@test.invalid','Owner','unused','owner')").bind(owner).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO products(id,title,slug) VALUES($1,'Shirt','shirt')")
        .bind(product)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO product_variants(id,product_id,title,sku,price_minor,currency) VALUES($1,$2,'Old red','EXISTING-SKU',1500,'EUR')").bind(legacy).bind(product).execute(&pool).await.unwrap();
    let parent: String = sqlx::query_scalar("SELECT sku FROM products WHERE id=$1")
        .bind(product)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(parent.starts_with("KP-"));
    let actor = AuthenticatedStaff {
        id: owner,
        email: "owner@test.invalid".into(),
        display_name: "Owner".into(),
        role: "owner".into(),
        capabilities: vec![],
    };
    let state = AppState {
        database: Some(pool.clone()),
        ..AppState::default()
    };
    let mut body = json!({"revision":0,"config":{"color_photos":false,"options":[{"code":"COL","label":"Color","control":"swatch","values":[{"code":"RED","label":"Red","color":"#ff0000"},{"code":"BLU","label":"Blue","color":"#0000ff"}]}]},"variants":[{"id":legacy,"selections":{"COL":"RED"},"price_minor":1500,"available_quantity":4,"expected_quantity":0,"retired":false},{"selections":{"COL":"BLU"},"price_minor":2000,"available_quantity":0,"retired":false}]});
    async fn save(s: &AppState, a: &AuthenticatedStaff, id: Uuid, b: &Value) -> StatusCode {
        save_matrix(
            State(s.clone()),
            a.clone(),
            Path(id),
            Json(serde_json::from_value::<SaveMatrix>(b.clone()).unwrap()),
        )
        .await
        .status()
    }
    let preview = knitnprint_api::variant_options::preview_matrix(
        State(state.clone()),
        actor.clone(),
        Path(product),
        Json(serde_json::from_value::<SaveMatrix>(body.clone()).unwrap()),
    )
    .await;
    assert_eq!(preview.status(), StatusCode::OK);
    let revision: i64 = sqlx::query_scalar("SELECT variant_revision FROM products WHERE id=$1")
        .bind(product)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(revision, 0);
    assert_eq!(save(&state, &actor, product, &body).await, StatusCode::OK);
    let (sku, sel): (String, Value) =
        sqlx::query_as("SELECT sku,selections FROM product_variants WHERE id=$1")
            .bind(legacy)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(sku, "EXISTING-SKU");
    assert_eq!(sel, json!({"COL":"RED"}));
    let (blue, blue_sku): (Uuid, String) =
        sqlx::query_as("SELECT id,sku FROM product_variants WHERE product_id=$1 AND id<>$2")
            .bind(product)
            .bind(legacy)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(blue_sku, format!("{parent}-COL-BLU"));
    assert_eq!(
        save(&state, &actor, product, &body).await,
        StatusCode::CONFLICT
    );
    body["revision"] = json!(1);
    body["variants"][0]["expected_quantity"] = json!(4);
    body["variants"][1]["id"] = json!(blue);
    body["variants"][1]["expected_quantity"] = json!(0);
    body["config"]["options"][0]["values"][0]["label"] = json!("Crimson");
    assert_eq!(save(&state, &actor, product, &body).await, StatusCode::OK);
    body["revision"] = json!(2);
    body["variants"][0]["expected_quantity"] = json!(3);
    assert_eq!(
        save(&state, &actor, product, &body).await,
        StatusCode::CONFLICT
    );
    body["variants"][0]["expected_quantity"] = json!(4);
    body["variants"][0]["selections"] = json!({"COL":"BLU"});
    body["variants"][1]["selections"] = json!({"COL":"RED"});
    assert_eq!(
        save(&state, &actor, product, &body).await,
        StatusCode::CONFLICT
    );
    assert!(
        sqlx::query("UPDATE product_variants SET sku='CHANGED' WHERE id=$1")
            .bind(legacy)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE products SET sku='CHANGED' WHERE id=$1")
            .bind(product)
            .execute(&pool)
            .await
            .is_err()
    );
    let unchanged: String = sqlx::query_scalar("SELECT sku FROM product_variants WHERE id=$1")
        .bind(blue)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(unchanged, blue_sku);
    // Retiring keeps the code reserved, and reactivation retains the same ID/SKU.
    body["variants"][0]["selections"] = json!({"COL":"RED"});
    body["variants"][1]["selections"] = json!({"COL":"BLU"});
    body["variants"][1]["retired"] = json!(true);
    assert_eq!(save(&state, &actor, product, &body).await, StatusCode::OK);
    let retired: bool = sqlx::query_scalar("SELECT retired FROM product_variants WHERE id=$1")
        .bind(blue)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(retired);
    sqlx::query("UPDATE products SET status='active',published_at=now() WHERE id=$1")
        .bind(product)
        .execute(&pool)
        .await
        .unwrap();
    let customer = Uuid::new_v4();
    let token = format!("variant-customer-{customer}");
    sqlx::query("INSERT INTO customers(id,customer_type,email,first_name,last_name) VALUES($1,'registered','variant@example.test','Variant','Tester')").bind(customer).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO customer_accounts(customer_id,password_hash,email_verified_at) VALUES($1,'fixture',now())").bind(customer).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO customer_sessions(id,customer_id,token_hash,expires_at) VALUES($1,$2,$3,now()+interval '1 day')").bind(Uuid::new_v4()).bind(customer).bind(sha2::Sha256::digest(token.as_bytes()).as_slice()).execute(&pool).await.unwrap();
    let unavailable = knitnprint_api::app(state.clone())
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/cart/items")
                .header("cookie", format!("knitnprint_customer={token}"))
                .header("content-type", "application/json")
                .header("idempotency-key", Uuid::new_v4().to_string())
                .body(axum::body::Body::from(
                    json!({"variant_id":blue,"quantity":1}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unavailable.status(), StatusCode::UNPROCESSABLE_ENTITY);

    body["revision"] = json!(3);
    body["variants"][1]["retired"] = json!(false);
    assert_eq!(save(&state, &actor, product, &body).await, StatusCode::OK);
    let stable: String = sqlx::query_scalar("SELECT sku FROM product_variants WHERE id=$1")
        .bind(blue)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stable, blue_sku);
    let mut denied = actor.clone();
    denied.role = "staff".into();
    assert_eq!(
        save(&state, &denied, product, &body).await,
        StatusCode::FORBIDDEN
    );
    // Photo resolution is opt-in, restricts mapped colors, and falls back for unmapped ones.
    let media = Uuid::new_v4();
    let other = Uuid::new_v4();
    let mut photos = body["config"].clone();
    photos["color_photos"] = json!(true);
    photos["options"][0]["values"][0]["media_ids"] = json!([media]);
    let views = json!([{"id":"red","media_id":media},{"id":"shared","media_id":other}]);
    let red: Value = sqlx::query_scalar("SELECT variant_views($1,$2,$3)")
        .bind(&photos)
        .bind(json!({"COL":"RED"}))
        .bind(&views)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(red, json!([{"id":"red","media_id":media}]));
    let blue_views: Value = sqlx::query_scalar("SELECT variant_views($1,$2,$3)")
        .bind(&photos)
        .bind(json!({"COL":"BLU"}))
        .bind(&views)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(blue_views, views);
    body["revision"] = json!(4);
    body["config"] = photos;
    assert_eq!(
        save(&state, &actor, product, &body).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let template_id = Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap();
    let mut definition: Value =
        sqlx::query_scalar("SELECT definition FROM variant_templates WHERE id=$1")
            .bind(template_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    definition["label"] = json!("Updated palette");
    let template_body = json!({"id":template_id,"revision":0,"definition":definition});
    let result = knitnprint_api::variant_options::save_template(
        State(state.clone()),
        actor.clone(),
        Path(template_id),
        Json(serde_json::from_value(template_body.clone()).unwrap()),
    )
    .await;
    assert_eq!(result.status(), StatusCode::OK);
    let stale = knitnprint_api::variant_options::save_template(
        State(state.clone()),
        actor.clone(),
        Path(template_id),
        Json(serde_json::from_value(template_body).unwrap()),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    let copied: Value = sqlx::query_scalar("SELECT variant_config FROM products WHERE id=$1")
        .bind(product)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(copied["options"][0]["label"], "Color");
    // Concurrent parent allocation produces distinct globally reserved identifiers.
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let (a, b) = tokio::join!(
        sqlx::query_scalar::<_, String>(
            "INSERT INTO products(id,title,slug) VALUES($1,'A','allocation-a') RETURNING sku"
        )
        .bind(first)
        .fetch_one(&pool),
        sqlx::query_scalar::<_, String>(
            "INSERT INTO products(id,title,slug) VALUES($1,'B','allocation-b') RETURNING sku"
        )
        .bind(second)
        .fetch_one(&pool)
    );
    assert_ne!(a.unwrap(), b.unwrap());
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
}
