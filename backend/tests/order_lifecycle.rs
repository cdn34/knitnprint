use std::{cell::RefCell, env, str::FromStr, sync::Arc};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use hmac::{Hmac, Mac};
use knitnprint_api::{
    AppState, app,
    auth::hash_password,
    email::EmailService,
    notifications::deliver_due,
    payments::{
        PaymentProvider, PaymentProviderError, PaymentService, ProviderCancelFuture,
        ProviderCheckout, ProviderCheckoutRequest, ProviderFuture, ProviderRefund,
        ProviderRefundFuture, ProviderRefundRequest,
    },
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tower::ServiceExt;
use uuid::Uuid;

thread_local! {
    static VERIFIED_CUSTOMER_COOKIE: RefCell<Option<String>> = const { RefCell::new(None) };
}

struct FakeStripe;

struct PreviewStorage;

#[async_trait::async_trait]
impl knitnprint_api::object_storage::StorageBackend for PreviewStorage {
    fn provider(&self) -> knitnprint_api::object_storage::StorageProvider {
        knitnprint_api::object_storage::StorageProvider::SeaweedFs
    }
    fn bucket(&self) -> &str {
        "test-previews"
    }
    async fn presign_upload(
        &self,
        _: &str,
        _: &str,
        _: i64,
        _: std::time::Duration,
    ) -> Result<String, knitnprint_api::object_storage::StorageError> {
        unreachable!()
    }
    async fn presign_download(
        &self,
        _: &str,
        _: std::time::Duration,
    ) -> Result<String, knitnprint_api::object_storage::StorageError> {
        unreachable!()
    }
    async fn head(
        &self,
        _: &str,
    ) -> Result<
        knitnprint_api::object_storage::ObjectMetadata,
        knitnprint_api::object_storage::StorageError,
    > {
        unreachable!()
    }
    async fn put(
        &self,
        _: &str,
        _: &str,
        _: Vec<u8>,
    ) -> Result<(), knitnprint_api::object_storage::StorageError> {
        unreachable!()
    }
    async fn delete(&self, _: &str) -> Result<(), knitnprint_api::object_storage::StorageError> {
        unreachable!()
    }
    async fn get(
        &self,
        _: &str,
    ) -> Result<
        knitnprint_api::object_storage::StoredObject,
        knitnprint_api::object_storage::StorageError,
    > {
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(96, 96)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        Ok(knitnprint_api::object_storage::StoredObject {
            bytes: png.into_inner(),
            content_type: Some("image/png".into()),
            etag: None,
        })
    }
}

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

    fn cancel_checkout(&self, _provider_payment_id: String) -> ProviderCancelFuture<'_> {
        Box::pin(async { Ok(()) })
    }

    fn refund(&self, request: ProviderRefundRequest) -> ProviderRefundFuture<'_> {
        Box::pin(async move {
            Ok(ProviderRefund {
                provider_refund_id: format!("re_test_{}", request.refund_id.simple()),
                status: "succeeded".to_owned(),
            })
        })
    }
}

#[tokio::test]
async fn checkout_requires_a_ready_cart_and_historical_manual_orders_remain_manageable() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    let schema = format!("checkout_retirement_{}", Uuid::new_v4().simple());
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\" "))
        .execute(&admin)
        .await
        .unwrap();
    let pool = isolated_pool(&url, &schema).await;
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let variant_id = insert_product(&pool).await;
    insert_owner(&pool).await;
    insert_order_reader(&pool).await;
    let email = EmailService::development("http://127.0.0.1:3000");
    let router = app(AppState {
        database: Some(pool.clone()),
        email: email.clone(),
        manual_payments_enabled: true,
        payments: PaymentService::with_provider(Arc::new(FakeStripe), "whsec_historical"),
        ..AppState::default()
    });
    create_verified_customer(&pool).await;
    for method in ["manual", "stripe"] {
        let rejected = request(
            &router,
            "POST",
            "/api/orders",
            None,
            Some(json!({"payment_method":method})),
            Some("checkout-paused-0001"),
        )
        .await;
        assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response_json(rejected).await["error"]["code"],
            "cart_not_ready"
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM orders")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let options =
        response_json(request(&router, "GET", "/api/payments/options", None, None, None).await)
            .await;
    assert_eq!(
        options,
        json!({"stripe":true,"manual":true,"checkout_suspended":false})
    );
    let owner = login(&router).await;
    for path in [
        "/api/admin/shipping-packages",
        "/api/cart/shipping-quotes",
        "/api/cart/shipping-method",
    ] {
        assert_eq!(
            request(&router, "POST", path, Some(&owner), Some(json!({})), None)
                .await
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    let settings = response_json(
        request(
            &router,
            "GET",
            "/api/admin/settings",
            Some(&owner),
            None,
            None,
        )
        .await,
    )
    .await;
    assert!(settings.get("shipping_zones").is_none());
    assert_eq!(settings["checkout_suspended"], false);
    assert!(settings["integrations"].get("packlink").is_none());
    let updated = request(&router,"POST","/api/admin/settings",Some(&owner),Some(json!({
        "store_name":"KnitNPrint","support_email":"geral@knitnprint.com","currency":"EUR","tax_enabled":false,
        "tax_rules":[],"reason":"Update identity without the retired delivery configuration"
    })),None).await;
    assert_eq!(updated.status(), StatusCode::OK);
    let created = request(&router,"POST","/api/admin/products",Some(&owner),Some(json!({
        "title":"No packaging required","slug":"no-packaging-required","description":"Product without obsolete delivery fields",
        "variants":[{"title":"Default","sku":"NO-DELIVERY-PROFILE","price_minor":2500,"currency":"EUR","option_values":{},"available_quantity":5}]
    })),None).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let product = response_json(created).await;
    assert!(product.get("shipping").is_none());
    let edited = request(&router,"PUT",&format!("/api/admin/products/{}",product["id"].as_str().unwrap()),Some(&owner),Some(json!({
        "title":"Edited without packaging","slug":"no-packaging-required","description":"Updated",
        "sku":"NO-DELIVERY-PROFILE","price_minor":2700,"currency":"EUR","available_quantity":8
    })),None).await;
    assert_eq!(edited.status(), StatusCode::OK);
    assert!(response_json(edited).await.get("shipping").is_none());

    let (_, id) = create_historical_order(
        &pool,
        &router,
        variant_id,
        "historical-manual",
        None,
        "manual",
    )
    .await;
    let pay_path = format!("/api/admin/orders/{id}/manual-payment");
    let reader = login_as(
        &router,
        "orders-reader@test.invalid",
        "integration-test-passphrase",
    )
    .await;
    assert_eq!(
        request(
            &router,
            "POST",
            &pay_path,
            Some(&reader),
            Some(json!({"reason":"Unauthorized payment"})),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let paid = request(
        &router,
        "POST",
        &pay_path,
        Some(&owner),
        Some(json!({"reason":"Historical payment received"})),
        None,
    )
    .await;
    assert_eq!(paid.status(), StatusCode::OK);
    let paid = response_json(paid).await;
    assert_eq!(paid["payment_status"], "paid");
    assert_eq!(paid["total_minor"], 6400);
    assert_eq!(
        request(
            &router,
            "POST",
            &pay_path,
            Some(&owner),
            Some(json!({"reason":"Historical payment received"})),
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(deliver_due(&pool, &email, 25).await.unwrap().sent, 1);
    let fulfilled = request(&router,"POST",&format!("/api/admin/orders/{id}/fulfillments"),Some(&owner),Some(json!({
        "carrier":"CTT","tracking_number":"HIST-ONE","tracking_url":"","reason":"Historical order dispatched",
        "lines":[{"order_line_id":paid["lines"][0]["id"],"quantity":1}]
    })),Some("historical-fulfillment-0001")).await;
    assert_eq!(fulfilled.status(), StatusCode::CREATED);
    assert_eq!(
        response_json(fulfilled).await["fulfillment_status"],
        "partially_fulfilled"
    );
    let refunded = request(&router,"POST",&format!("/api/admin/orders/{id}/refunds"),Some(&owner),Some(json!({
        "mode":"full","lines":[],"restock":true,"reason":"Historical return accepted","internal_note":"Historical operations remain available"
    })),Some("historical-refund-0001")).await;
    assert_eq!(refunded.status(), StatusCode::OK);
    assert_eq!(response_json(refunded).await["payment_status"], "refunded");
    let quantities: (i64,i64,i64) = sqlx::query_as("SELECT available_quantity,reserved_quantity,committed_quantity FROM inventory_items WHERE variant_id=$1")
        .bind(variant_id).fetch_one(&pool).await.unwrap();
    assert_eq!(quantities, (5, 0, 0));
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
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
    insert_owner(&pool).await;
    let webhook_secret = "whsec_order_lifecycle";
    let router = app(AppState {
        database: Some(pool.clone()),
        payments: PaymentService::with_provider(Arc::new(FakeStripe), webhook_secret),
        ..AppState::default()
    });
    create_verified_customer(&pool).await;

    let (cart_cookie, order_id) =
        create_stripe_order(&pool, &router, variant_id, "paid", None).await;
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
        create_stripe_order(&pool, &router, variant_id, "expired", Some(fresh_cookie)).await;
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

    let owner_cookie = login(&router).await;
    let failed_dashboard = request(
        &router,
        "GET",
        "/api/admin/dashboard",
        Some(&owner_cookie),
        None,
        None,
    )
    .await;
    let failed_dashboard_body = response_json(failed_dashboard).await;
    assert_eq!(failed_dashboard_body["metrics"]["failed_payments"], 1);
    assert_eq!(
        failed_dashboard_body["failed_payments"][0]["order_id"],
        expired_order_id
    );
    let stripe_refund = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/refunds"),
        Some(&owner_cookie),
        Some(json!({
            "mode": "full",
            "lines": [],
            "restock": true,
            "reason": "Stripe test refund",
            "internal_note": "Provider adapter lifecycle"
        })),
        Some("stripe-refund-full-0001"),
    )
    .await;
    assert_eq!(stripe_refund.status(), StatusCode::OK);
    let stripe_refund_body = response_json(stripe_refund).await;
    assert_eq!(stripe_refund_body["payment_status"], "refunded");
    assert_eq!(stripe_refund_body["order_status"], "cancelled");
    assert!(
        stripe_refund_body["refunds"][0]["provider_refund_id"]
            .as_str()
            .unwrap()
            .starts_with("re_test_")
    );
    let customer_refund_view = request(
        &router,
        "GET",
        &format!("/api/orders/{order_id}"),
        Some(&cart_cookie),
        None,
        None,
    )
    .await;
    assert!(response_json(customer_refund_view).await["refunds"][0]["internal_note"].is_null());
    let quantities: (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, committed_quantity FROM inventory_items WHERE variant_id = $1",
    )
    .bind(variant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quantities, (7, 0, 0));

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
    let (_, abandoned_order_id) = create_stripe_order(
        &pool,
        &router,
        variant_id,
        "abandoned",
        Some(third_cookie.clone()),
    )
    .await;
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
        knitnprint_api::payments::cleanup_abandoned(&pool, 100)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        knitnprint_api::payments::cleanup_abandoned(&pool, 100)
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
    assert_eq!(quantities, (7, 0, 0));

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
async fn checkout_saves_locale_and_sends_localized_branded_confirmation_once() {
    let Ok(url) = env::var("DATABASE_URL") else {
        eprintln!("skipping PostgreSQL integration test: DATABASE_URL not set");
        return;
    };
    let schema = format!("localized_checkout_{}", Uuid::new_v4().simple());
    let admin = PgPool::connect(&url).await.unwrap();
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&admin)
        .await
        .unwrap();
    let pool = isolated_pool(&url, &schema).await;
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let variant = insert_product(&pool).await;
    insert_owner(&pool).await;
    create_verified_customer(&pool).await;
    let email = EmailService::development("https://shop.example.test");
    let router = app(AppState {
        database: Some(pool.clone()),
        email: email.clone(),
        manual_payments_enabled: true,
        ..AppState::default()
    });
    let owner = login(&router).await;
    for (index, language, expected, heading) in [
        (0, "pt", "pt", "A tua encomenda está confirmada."),
        (1, "es", "es", "Tu pedido está confirmado."),
        (2, "en", "en", "Your order is confirmed."),
    ] {
        let cookie = response_cookie(&request(&router, "GET", "/api/cart", None, None, None).await);
        let added = request(
            &router,
            "POST",
            "/api/cart/items",
            Some(&cookie),
            Some(json!({"variant_id":variant,"quantity":1})),
            Some(&format!("localized-add-{index}-0001")),
        )
        .await;
        assert_eq!(added.status(), StatusCode::CREATED);
        let delivered = request(
            &router,
            "POST",
            "/api/cart/delivery",
            Some(&cookie),
            Some(delivery_fixture()),
            Some(&format!("localized-delivery-{index}-0001")),
        )
        .await;
        assert_eq!(response_json(delivered).await["checkout_ready"], true);
        let preview_ids = [Uuid::new_v4(), Uuid::new_v4()];
        if index == 0 {
            let cart_id: Uuid = sqlx::query_scalar("SELECT id FROM carts WHERE token_hash=$1")
                .bind(
                    Sha256::digest(cookie.strip_prefix("knitnprint_cart=").unwrap().as_bytes())
                        .as_slice(),
                )
                .fetch_one(&pool)
                .await
                .unwrap();
            for id in preview_ids {
                sqlx::query("INSERT INTO media_assets (id,object_key,content_type,byte_size,status,scan_status,scanned_at,personalization_cart_id) VALUES ($1,$2,'image/png',100,'ready','clean',now(),$3)")
                    .bind(id).bind(format!("private-preview-{id}.png")).bind(cart_id).execute(&pool).await.unwrap();
                sqlx::query("INSERT INTO media_variants (media_asset_id,kind,object_key,content_type,width,height,byte_size) VALUES ($1,'card',$2,'image/png',96,96,100)").bind(id).bind(format!("saved-preview-{id}.png")).execute(&pool).await.unwrap();
            }
            sqlx::query("UPDATE cart_lines SET customization=$2,personalization_previews=$3 WHERE cart_id=$1")
                .bind(cart_id).bind(json!({"version":7,"areas":[{"view_id":"front","view_label":"Front","text":{"content":"<script>Maria & João</script>"}},{"view_id":"back","view_label":"Back","text":{"content":"Made for you"}}],"previews":[{"view_id":"front","view_label":"Front","media_id":preview_ids[0]},{"view_id":"back","view_label":"Back","media_id":preview_ids[1]}]}))
                .bind(json!([{"view_id":"front","view_label":"Front","media_id":preview_ids[0]},{"view_id":"back","view_label":"Back","media_id":preview_ids[1]}])).execute(&pool).await.unwrap();
        }
        let key = format!("localized-order-{index}-0001");
        let created = request(
            &router,
            "POST",
            "/api/orders",
            Some(&cookie),
            Some(json!({"payment_method":"manual","locale":language})),
            Some(&key),
        )
        .await;
        let status = created.status();
        let order = response_json(created).await;
        assert_eq!(status, StatusCode::CREATED, "{order}");
        assert_eq!(order["shipping_minor"], 0);
        assert_eq!(order["total_minor"], 3200);
        let id = Uuid::parse_str(order["id"].as_str().unwrap()).unwrap();
        let saved: String = sqlx::query_scalar("SELECT email_locale FROM orders WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(saved, expected);
        let replay = request(
            &router,
            "POST",
            "/api/orders",
            Some(&cookie),
            Some(json!({"payment_method":"manual","locale":"es"})),
            Some(&key),
        )
        .await;
        assert_eq!(replay.status(), StatusCode::OK);
        assert_eq!(response_json(replay).await["id"], order["id"]);
        assert!(
            sqlx::query("UPDATE orders SET email_locale=$2 WHERE id=$1")
                .bind(id)
                .bind(if expected == "es" { "pt" } else { "es" })
                .execute(&pool)
                .await
                .is_err()
        );
        let pay_path = format!("/api/admin/orders/{id}/manual-payment");
        for _ in 0..2 {
            assert_eq!(
                request(
                    &router,
                    "POST",
                    &pay_path,
                    Some(&owner),
                    Some(json!({"reason":"Confirmed test payment"})),
                    None
                )
                .await
                .status(),
                StatusCode::OK
            );
        }
        let storage =
            knitnprint_api::object_storage::ObjectStorage::from_backend(Arc::new(PreviewStorage));
        assert_eq!(
            knitnprint_api::notifications::deliver_due_with_storage(
                &pool,
                &email,
                25,
                Some(&storage)
            )
            .await
            .unwrap()
            .sent,
            1
        );
        assert_eq!(deliver_due(&pool, &email, 25).await.unwrap().sent, 0);
        let mailbox = response_json(
            request(
                &router,
                "GET",
                "/api/development/emails/latest?to=order%40example.com&kind=order_confirmation",
                None,
                None,
                None,
            )
            .await,
        )
        .await;
        let html = mailbox["html"].as_str().unwrap();
        assert!(html.contains(heading));
        assert!(html.contains(&format!("lang=\"{expected}\"")));
        assert!(html.contains("cid:wordmark@knitnprint"));
        assert!(html.contains("#f7f3ef"));
        assert!(html.contains("support@knitnprint.com"));
        assert!(html.contains(&format!("order_id={id}")));
        assert!(!html.contains("manual") && !html.contains("progress"));
        assert!(!html.contains("<script>"));
        if index == 0 {
            assert!(html.contains("&lt;script&gt;Maria &amp; João&lt;/script&gt;"));
            assert_eq!(
                order["lines"][0]["customization_media_asset_ids"],
                json!([]),
                "email previews must not be treated as production originals"
            );
            assert!(html.contains("alt=\"Front\"") && html.contains("alt=\"Back\""));
            let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM order_line_personalization_media retained JOIN order_lines line ON line.id=retained.order_line_id WHERE line.order_id=$1")
                .bind(id).fetch_one(&pool).await.unwrap();
            assert_eq!(retained, 2);
        }
        if let Ok(directory) = env::var("KNITNPRINT_EMAIL_PREVIEW_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("confirmation-{expected}.html")),
                html,
            )
            .unwrap();
        }
        assert!(mailbox["text"].as_str().unwrap().contains(heading));
    }
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
async fn editable_destination_rates_control_cart_checkout_and_immutable_order_prices() {
    let Ok(url) = env::var("DATABASE_URL") else {
        return;
    };
    let schema = format!("delivery_rates_{}", Uuid::new_v4().simple());
    let admin = PgPool::connect(&url).await.unwrap();
    sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
        .execute(&admin)
        .await
        .unwrap();
    let pool = isolated_pool(&url, &schema).await;
    sqlx::migrate!("../migrations").run(&pool).await.unwrap();
    let variant = insert_product(&pool).await;
    insert_owner(&pool).await;
    insert_order_reader(&pool).await;
    create_verified_customer(&pool).await;
    let router = app(AppState {
        database: Some(pool.clone()),
        manual_payments_enabled: true,
        ..AppState::default()
    });
    let owner = login(&router).await;
    let reader = login_as(
        &router,
        "orders-reader@test.invalid",
        "integration-test-passphrase",
    )
    .await;
    for (method, path, body) in [
        ("GET", "/api/admin/delivery-rates", None),
        (
            "PUT",
            "/api/admin/delivery-rates",
            Some(json!({"enabled":true,"revision":0,"rates":[],"reason":"Access test"})),
        ),
        (
            "POST",
            "/api/admin/delivery-rates/test",
            Some(json!({"country_code":"PT","postal_code":"9000-001"})),
        ),
    ] {
        assert_eq!(
            request(&router, method, path, Some(&reader), body, None)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    let mut settings = response_json(
        request(
            &router,
            "GET",
            "/api/admin/delivery-rates",
            Some(&owner),
            None,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(settings["enabled"], false);
    assert_eq!(settings["rates"].as_array().unwrap().len(), 4);
    assert!(
        settings["rates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|rate| rate["amount_minor"].is_null() && rate["active"] == false)
    );
    let mut invalid = settings.clone();
    invalid["enabled"] = json!(true);
    invalid["rates"][0]["active"] = json!(true);
    invalid["reason"] = json!("Reject unpriced destination");
    assert_eq!(
        request(
            &router,
            "PUT",
            "/api/admin/delivery-rates",
            Some(&owner),
            Some(invalid),
            None
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for (index, amount) in [450, 800, 900, 650].iter().enumerate() {
        settings["rates"][index]["amount_minor"] = json!(amount);
        settings["rates"][index]["active"] = json!(true);
    }
    settings["enabled"] = json!(true);
    settings["reason"] = json!("Configure initial shipping destinations");
    let saved = request(
        &router,
        "PUT",
        "/api/admin/delivery-rates",
        Some(&owner),
        Some(settings.clone()),
        None,
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    let mut saved = response_json(saved).await;
    assert_eq!(saved["revision"], 1);
    assert_eq!(
        request(
            &router,
            "PUT",
            "/api/admin/delivery-rates",
            Some(&owner),
            Some(settings.clone()),
            None
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    for (country, postal, amount, region) in [
        ("PT", "1000-001", 450, "Mainland Portugal"),
        ("PT", "8999-999", 450, "Mainland Portugal"),
        ("PT", "9000-001", 800, "Madeira"),
        ("PT", "9499-999", 800, "Madeira"),
        ("PT", "9500-001", 900, "Azores"),
        ("PT", "9999-999", 900, "Azores"),
        ("ES", "28001", 650, "All regions"),
    ] {
        let quote = response_json(
            request(
                &router,
                "POST",
                "/api/admin/delivery-rates/test",
                Some(&owner),
                Some(json!({"country_code":country,"postal_code":postal})),
                None,
            )
            .await,
        )
        .await;
        assert_eq!(quote["available"], true, "{quote}");
        assert_eq!(quote["amount_minor"], amount);
        assert_eq!(quote["region_name"], region);
    }
    let cookie = response_cookie(&request(&router, "GET", "/api/cart", None, None, None).await);
    assert_eq!(
        request(
            &router,
            "POST",
            "/api/cart/items",
            Some(&cookie),
            Some(json!({"variant_id":variant,"quantity":1})),
            Some("delivery-rate-add-0001")
        )
        .await
        .status(),
        StatusCode::CREATED
    );
    let mut delivery = delivery_fixture();
    delivery["address"]["postal_code"] = json!("9000-001");
    delivery["address"]["region"] = json!("Mainland Portugal");
    let cart = response_json(
        request(
            &router,
            "POST",
            "/api/cart/delivery",
            Some(&cookie),
            Some(delivery.clone()),
            Some("delivery-rate-address-0001"),
        )
        .await,
    )
    .await;
    assert_eq!(cart["shipping_minor"], 800);
    assert_eq!(cart["total_minor"], 4000);
    assert_eq!(cart["shipping_quote"]["region_name"], "Madeira");
    // A change between cart review and checkout must not silently charge a new price.
    saved["rates"][1]["amount_minor"] = json!(1000);
    saved["reason"] = json!("Update Madeira shipping price");
    let revised = request(
        &router,
        "PUT",
        "/api/admin/delivery-rates",
        Some(&owner),
        Some(saved),
        None,
    )
    .await;
    assert_eq!(revised.status(), StatusCode::OK);
    let mut revised = response_json(revised).await;
    let stale = request(
        &router,
        "POST",
        "/api/orders",
        Some(&cookie),
        Some(json!({"payment_method":"manual","shipping_rate_revision":1})),
        Some("delivery-rate-checkout-0001"),
    )
    .await;
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(response_json(stale).await["error"]["code"], "cart_changed");
    let cart =
        response_json(request(&router, "GET", "/api/cart", Some(&cookie), None, None).await).await;
    assert_eq!(cart["shipping_minor"], 1000);
    assert_eq!(cart["total_minor"], 4200);
    let order = request(
        &router,
        "POST",
        "/api/orders",
        Some(&cookie),
        Some(json!({"payment_method":"manual","locale":"pt","shipping_rate_revision":2})),
        Some("delivery-rate-checkout-0001"),
    )
    .await;
    let status = order.status();
    let order = response_json(order).await;
    assert_eq!(status, StatusCode::CREATED, "{order}");
    assert_eq!(order["shipping_minor"], 1000);
    assert_eq!(order["shipping"]["zone_name"], "Madeira");
    assert_eq!(order["total_minor"], 4200);
    let order_id = order["id"].as_str().unwrap();
    revised["rates"][1]["active"] = json!(false);
    revised["rates"].as_array_mut().unwrap().push(json!({"id":Uuid::new_v4(),"country_code":"FR","region_name":"All regions","postal_prefixes":[],"amount_minor":700,"active":true}));
    revised["rates"].as_array_mut().unwrap().push(json!({"id":Uuid::new_v4(),"country_code":"PT","region_name":"Whole country fallback","postal_prefixes":[],"amount_minor":100,"active":true}));
    revised["reason"] = json!("Disable Madeira and add France");
    let changed = request(
        &router,
        "PUT",
        "/api/admin/delivery-rates",
        Some(&owner),
        Some(revised),
        None,
    )
    .await;
    assert_eq!(changed.status(), StatusCode::OK);
    let mut changed = response_json(changed).await;
    let historical = response_json(
        request(
            &router,
            "GET",
            &format!("/api/orders/{order_id}"),
            Some(&cookie),
            None,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(historical["shipping_minor"], 1000);
    assert_eq!(historical["total_minor"], 4200);
    let quote = response_json(
        request(
            &router,
            "POST",
            "/api/admin/delivery-rates/test",
            Some(&owner),
            Some(json!({"country_code":"FR","postal_code":"75001"})),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(quote["amount_minor"], 700);
    let cookie2 = response_cookie(&request(&router, "GET", "/api/cart", None, None, None).await);
    assert_eq!(
        request(
            &router,
            "POST",
            "/api/cart/items",
            Some(&cookie2),
            Some(json!({"variant_id":variant,"quantity":1})),
            Some("delivery-rate-add-0002")
        )
        .await
        .status(),
        StatusCode::CREATED
    );
    let unavailable = response_json(
        request(
            &router,
            "POST",
            "/api/cart/delivery",
            Some(&cookie2),
            Some(delivery),
            Some("delivery-rate-address-0002"),
        )
        .await,
    )
    .await;
    assert_eq!(unavailable["checkout_ready"], false);
    assert!(unavailable["shipping_minor"].is_null());
    assert_eq!(
        unavailable["issues"][0]["code"],
        "shipping_destination_unavailable"
    );
    let blocked = request(
        &router,
        "POST",
        "/api/orders",
        Some(&cookie2),
        Some(json!({"payment_method":"manual","shipping_rate_revision":3})),
        Some("delivery-rate-blocked-0001"),
    )
    .await;
    assert_eq!(blocked.status(), StatusCode::UNPROCESSABLE_ENTITY);
    changed["enabled"] = json!(false);
    changed["reason"] = json!("Disable destination pricing temporarily");
    assert_eq!(
        request(
            &router,
            "PUT",
            "/api/admin/delivery-rates",
            Some(&owner),
            Some(changed),
            None
        )
        .await
        .status(),
        StatusCode::OK
    );
    let free =
        response_json(request(&router, "GET", "/api/cart", Some(&cookie2), None, None).await).await;
    assert_eq!(free["shipping_minor"], 0);
    assert_eq!(free["total_minor"], 3200);
    assert_eq!(free["checkout_ready"], true);
    let audited: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action='delivery_rates.update'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audited, 4);
    pool.close().await;
    sqlx::query(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

async fn create_stripe_order(
    pool: &PgPool,
    router: &axum::Router,
    variant_id: Uuid,
    suffix: &str,
    cookie: Option<String>,
) -> (String, String) {
    create_historical_order(pool, router, variant_id, suffix, cookie, "stripe").await
}

async fn create_historical_order(
    pool: &PgPool,
    router: &axum::Router,
    variant_id: Uuid,
    suffix: &str,
    cookie: Option<String>,
    provider: &str,
) -> (String, String) {
    let cookie = match cookie {
        Some(cookie) => cookie,
        None => response_cookie(&request(router, "GET", "/api/cart", None, None, None).await),
    };
    let added = request(
        router,
        "POST",
        "/api/cart/items",
        Some(&cookie),
        Some(json!({"variant_id":variant_id,"quantity":2})),
        Some(&format!("fixture-add-{suffix}-0001")),
    )
    .await;
    assert_eq!(added.status(), StatusCode::CREATED);
    let cart = response_json(added).await;
    let cart_id = Uuid::parse_str(cart["id"].as_str().unwrap()).unwrap();
    let delivered = request(
        router,
        "POST",
        "/api/cart/delivery",
        Some(&cookie),
        Some(delivery_fixture()),
        Some(&format!("fixture-delivery-{suffix}-0001")),
    )
    .await;
    assert_eq!(delivered.status(), StatusCode::OK);
    let id = Uuid::now_v7();
    let payment_id = Uuid::now_v7();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(r#"INSERT INTO orders (id,order_number,cart_id,customer_id,checkout_idempotency_hash,currency,
        subtotal_minor,total_minor,customer_email,customer_first_name,customer_last_name,shipping_recipient_name,
        shipping_line1,shipping_city,shipping_postal_code,shipping_country_code)
        SELECT $1,'HIST-' || $1::text,cart.id,customer.id,$3,'EUR',6400,6400,customer.email,customer.first_name,
        customer.last_name,'Ada Loom','8 Timeline Lane','Lisbon','1000-008','PT'
        FROM carts cart JOIN customers customer ON customer.id=cart.customer_id WHERE cart.id=$2"#)
        .bind(id).bind(cart_id).bind(Sha256::digest(suffix.as_bytes()).to_vec()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO order_lines (id,order_id,product_id,variant_id,product_title,variant_title,sku,quantity,unit_price_minor,line_total_minor,currency,position) SELECT $1,$2,product_id,id,'Order Loom',title,sku,2,3200,6400,currency,0 FROM product_variants WHERE id=$3")
        .bind(Uuid::now_v7()).bind(id).bind(variant_id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO order_shipping_snapshots (order_id,zone_name,method_name,country_code,amount_minor,currency) VALUES ($1,'Historical zone','Historical delivery','PT',0,'EUR')")
        .bind(id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO order_tax_snapshots (order_id,rule_name,country_code,rate_basis_points,taxable_amount_minor,amount_minor,behavior) VALUES ($1,'Tax disabled','PT',0,6400,0,'disabled')")
        .bind(id).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO order_payments (id,order_id,provider,amount_minor,currency) VALUES ($1,$2,$3,6400,'EUR')")
        .bind(payment_id).bind(id).bind(provider).execute(&mut *tx).await.unwrap();
    if provider == "stripe" {
        let session = format!("cs_test_{}", id.simple());
        let attempt_id = Uuid::now_v7();
        sqlx::query("INSERT INTO payment_attempts (id,order_payment_id,attempt_number,provider,provider_payment_id,status,checkout_url,expires_at) VALUES ($1,$2,1,'stripe',$3,'pending',$4,now()+interval '30 minutes')")
            .bind(attempt_id).bind(payment_id).bind(&session).bind(format!("https://checkout.stripe.test/{id}")).execute(&mut *tx).await.unwrap();
        sqlx::query("UPDATE order_payments SET provider_payment_id=$2 WHERE id=$1")
            .bind(payment_id)
            .bind(&session)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO payment_status_events (id,order_payment_id,payment_attempt_id,provider,event_type,provider_status) VALUES ($1,$2,$3,'stripe','payment.checkout_created','pending')")
            .bind(Uuid::now_v7()).bind(payment_id).bind(attempt_id).execute(&mut *tx).await.unwrap();
    }
    sqlx::query("INSERT INTO order_events (id,order_id,event_type,title) VALUES ($1,$2,'order.created','Historical order created')")
        .bind(Uuid::now_v7()).bind(id).execute(&mut *tx).await.unwrap();
    knitnprint_api::inventory::reserve_in_transaction(
        &mut tx,
        variant_id,
        2,
        "Historical order reservation",
    )
    .await
    .unwrap();
    sqlx::query("UPDATE carts SET status='converted' WHERE id=$1")
        .bind(cart_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    (cookie, id.to_string())
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
                "payment_intent": format!("pi_test_{}", order_id.replace('-', "")),
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

async fn insert_order_reader(pool: &PgPool) {
    let password_hash = hash_password("integration-test-passphrase").unwrap();
    let staff_id = Uuid::now_v7();
    sqlx::query("INSERT INTO staff_users (id, email, display_name, password_hash, role) VALUES ($1, 'orders-reader@test.invalid', 'Order Reader', $2, 'staff')")
        .bind(staff_id)
        .bind(password_hash)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO staff_capabilities (staff_user_id, capability_name) VALUES ($1, 'orders.read')")
        .bind(staff_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn login(router: &axum::Router) -> String {
    login_as(
        router,
        "orders-owner@test.invalid",
        "integration-test-passphrase",
    )
    .await
}

async fn login_as(router: &axum::Router, email: &str, password: &str) -> String {
    let response = request(
        router,
        "POST",
        "/api/admin/auth/login",
        None,
        Some(json!({
            "email": email,
            "password": password
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

async fn create_verified_customer(pool: &PgPool) {
    let customer_id = Uuid::now_v7();
    let token = format!("order-test-customer-{customer_id}");
    let token_hash = sha2::Sha256::digest(token.as_bytes());
    sqlx::query("INSERT INTO customers (id, customer_type, email, first_name, last_name) VALUES ($1, 'registered', 'order@example.com', 'Ada', 'Loom')")
        .bind(customer_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO customer_accounts (customer_id, password_hash, email_verified_at) VALUES ($1, 'test-fixture', now())")
        .bind(customer_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO customer_sessions (id, customer_id, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 day')")
        .bind(Uuid::now_v7()).bind(customer_id).bind(token_hash.as_slice()).execute(pool).await.unwrap();
    VERIFIED_CUSTOMER_COOKIE
        .with(|cookie| *cookie.borrow_mut() = Some(format!("knitnprint_customer={token}")));
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
    let cookie = VERIFIED_CUSTOMER_COOKIE.with(|verified| {
        let verified = verified.borrow();
        match (verified.as_deref(), cookie) {
            (Some(account), Some(other)) => Some(format!("{account}; {other}")),
            (Some(account), None) => Some(account.to_owned()),
            (None, Some(other)) => Some(other.to_owned()),
            _ => None,
        }
    });
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
