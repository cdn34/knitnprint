use std::{env, str::FromStr, sync::Arc};

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use hmac::{Hmac, Mac};
use knitprint_api::{
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
    insert_order_reader(&pool).await;
    let email = EmailService::development("http://127.0.0.1:3000");
    let router = app(AppState {
        database: Some(pool.clone()),
        email: email.clone(),
        manual_payments_enabled: true,
        ..AppState::default()
    });
    let owner_cookie = login(&router).await;
    let configured = request(
        &router,
        "POST",
        "/api/admin/settings",
        Some(&owner_cookie),
        Some(json!({
            "store_name": "KnitPrint Test Studio",
            "support_email": "support@example.com",
            "currency": "eur",
            "tax_enabled": true,
            "shipping_zones": [{
                "name": "Portugal",
                "country_codes": ["pt"],
                "active": true,
                "methods": [
                    { "name": "Standard tracked", "flat_rate_minor": 500, "active": true },
                    { "name": "Express tracked", "flat_rate_minor": 900, "active": true }
                ]
            }],
            "tax_rules": [{
                "name": "Test destination tax",
                "country_codes": ["pt"],
                "rate_basis_points": 2300,
                "active": true
            }],
            "reason": "Configure deterministic order lifecycle pricing"
        })),
        None,
    )
    .await;
    assert_eq!(configured.status(), StatusCode::OK);
    let configured_body = response_json(configured).await;
    assert_eq!(configured_body["currency"], "EUR");
    assert_eq!(
        configured_body["integrations"]["email"],
        "development_mailbox"
    );
    assert_eq!(
        configured_body["integrations"]["payments"],
        "manual_development"
    );
    let discount = request(
        &router,
        "POST",
        "/api/admin/discounts",
        Some(&owner_cookie),
        Some(json!({
            "code": "order10",
            "kind": "percentage",
            "value": 1000,
            "currency": "eur",
            "minimum_order_minor": 5000,
            "starts_at": null,
            "ends_at": null,
            "usage_limit": 2,
            "per_customer_limit": 1,
            "reason": "Order lifecycle promotion"
        })),
        None,
    )
    .await;
    assert_eq!(discount.status(), StatusCode::CREATED);
    let discount_id = response_json(discount).await["id"]
        .as_str()
        .unwrap()
        .to_owned();

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
    let delivered_body = response_json(delivered).await;
    assert_eq!(
        delivered_body["shipping_methods"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        delivered_body["shipping"]["method_name"],
        "Standard tracked"
    );
    let express_method_id = delivered_body["shipping_methods"][1]["id"]
        .as_str()
        .unwrap();
    let selected_shipping = request(
        &router,
        "POST",
        "/api/cart/shipping-method",
        Some(&cart_cookie),
        Some(json!({ "shipping_method_id": express_method_id })),
        Some("order-shipping-method-0001"),
    )
    .await;
    assert_eq!(selected_shipping.status(), StatusCode::OK);
    assert_eq!(
        response_json(selected_shipping).await["shipping"]["method_name"],
        "Express tracked"
    );
    let discounted = request(
        &router,
        "POST",
        "/api/cart/discount",
        Some(&cart_cookie),
        Some(json!({ "code": " order10 " })),
        Some("order-discount-apply-0001"),
    )
    .await;
    assert_eq!(discounted.status(), StatusCode::OK);
    let discounted_body = response_json(discounted).await;
    assert_eq!(discounted_body["subtotal_minor"], 6400);
    assert_eq!(discounted_body["discount_minor"], 640);
    assert_eq!(discounted_body["shipping_minor"], 900);
    assert_eq!(discounted_body["tax_minor"], 1531);
    assert_eq!(discounted_body["total_minor"], 8191);
    assert_eq!(discounted_body["discount"]["code"], "ORDER10");

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
    assert_eq!(created_body["subtotal_minor"], 6400);
    assert_eq!(created_body["discount_minor"], 640);
    assert_eq!(created_body["shipping_minor"], 900);
    assert_eq!(created_body["shipping"]["method_name"], "Express tracked");
    assert_eq!(created_body["tax_minor"], 1531);
    assert_eq!(created_body["tax"]["rate_basis_points"], 2300);
    assert_eq!(created_body["total_minor"], 8191);
    assert_eq!(created_body["discount"]["code"], "ORDER10");
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

    let reconfigured = request(
        &router,
        "POST",
        "/api/admin/settings",
        Some(&owner_cookie),
        Some(json!({
            "store_name": "KnitPrint Test Studio",
            "support_email": "support@example.com",
            "currency": "EUR",
            "tax_enabled": false,
            "shipping_zones": [{
                "name": "Worldwide",
                "country_codes": [],
                "active": true,
                "methods": [{ "name": "Free shipping", "flat_rate_minor": 0, "active": true }]
            }],
            "tax_rules": [],
            "reason": "Verify historical commercial snapshots remain stable"
        })),
        None,
    )
    .await;
    assert_eq!(reconfigured.status(), StatusCode::OK);

    let disabled = request(
        &router,
        "POST",
        &format!("/api/admin/discounts/{discount_id}/status"),
        Some(&owner_cookie),
        Some(json!({ "enabled": false, "reason": "Promotion window closed" })),
        None,
    )
    .await;
    assert_eq!(disabled.status(), StatusCode::OK);
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
    assert_eq!(detail_body["discount"]["code"], "ORDER10");
    assert_eq!(detail_body["discount"]["amount_minor"], 640);
    assert_eq!(detail_body["shipping"]["method_name"], "Express tracked");
    assert_eq!(detail_body["shipping_minor"], 900);
    assert_eq!(detail_body["tax"]["rule_name"], "Test destination tax");
    assert_eq!(detail_body["tax_minor"], 1531);
    assert_eq!(detail_body["total_minor"], 8191);
    let settings_history_count: i64 = sqlx::query_scalar("SELECT count(*) FROM settings_history")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(settings_history_count, 2);
    let usage_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM discount_usages WHERE order_id = $1")
            .bind(Uuid::parse_str(order_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(usage_count, 1);

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
    assert_eq!(paid_body["notifications"][0]["kind"], "order_confirmation");
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

    let line_id = paid_body["lines"][0]["id"].as_str().unwrap();
    let reader_cookie = login_as(
        &router,
        "orders-reader@test.invalid",
        "integration-test-passphrase",
    )
    .await;
    assert_eq!(
        request(
            &router,
            "GET",
            "/api/admin/discounts",
            Some(&reader_cookie),
            None,
            None,
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &router,
            "GET",
            "/api/admin/settings",
            Some(&reader_cookie),
            None,
            None,
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let denied = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/fulfillments"),
        Some(&reader_cookie),
        Some(json!({
            "carrier": "",
            "tracking_number": "",
            "tracking_url": "",
            "reason": "Unauthorized shipment",
            "lines": [{ "order_line_id": line_id, "quantity": 1 }]
        })),
        Some("order-fulfillment-denied-0001"),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let partial = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/fulfillments"),
        Some(&owner_cookie),
        Some(json!({
            "carrier": "CTT",
            "tracking_number": "TRACK-ONE",
            "tracking_url": "https://tracking.example.test/TRACK-ONE",
            "reason": "First parcel dispatched",
            "lines": [{ "order_line_id": line_id, "quantity": 1 }]
        })),
        Some("order-fulfillment-0001"),
    )
    .await;
    assert_eq!(partial.status(), StatusCode::CREATED);
    let partial_body = response_json(partial).await;
    assert_eq!(partial_body["order_status"], "confirmed");
    assert_eq!(partial_body["fulfillment_status"], "partially_fulfilled");
    assert_eq!(partial_body["lines"][0]["fulfilled_quantity"], 1);
    assert_eq!(partial_body["fulfillments"][0]["carrier"], "CTT");
    assert_eq!(partial_body["notifications"].as_array().unwrap().len(), 2);

    let replay = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/fulfillments"),
        Some(&owner_cookie),
        Some(json!({
            "carrier": "CTT",
            "tracking_number": "TRACK-ONE",
            "tracking_url": "https://tracking.example.test/TRACK-ONE",
            "reason": "First parcel dispatched",
            "lines": [{ "order_line_id": line_id, "quantity": 1 }]
        })),
        Some("order-fulfillment-0001"),
    )
    .await;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(
        response_json(replay).await["fulfillments"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let conflict = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/fulfillments"),
        Some(&owner_cookie),
        Some(json!({
            "carrier": "CTT",
            "tracking_number": "TRACK-TWO",
            "tracking_url": "",
            "reason": "Different request",
            "lines": [{ "order_line_id": line_id, "quantity": 1 }]
        })),
        Some("order-fulfillment-0001"),
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let failed_delivery = deliver_due(&pool, &EmailService::default(), 25)
        .await
        .unwrap();
    assert_eq!(failed_delivery.failed, 2);
    let status: (String, String) =
        sqlx::query_as("SELECT order_status, fulfillment_status FROM orders WHERE id = $1")
            .bind(Uuid::parse_str(order_id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, ("confirmed".into(), "partially_fulfilled".into()));
    sqlx::query("UPDATE notification_jobs SET next_attempt_at = now() WHERE status = 'pending'")
        .execute(&pool)
        .await
        .unwrap();
    let delivered = deliver_due(&pool, &email, 25).await.unwrap();
    assert_eq!(delivered.sent, 2);
    let fulfillment_email = request(
        &router,
        "GET",
        "/api/development/emails/latest?to=order%40example.com&kind=fulfillment_created",
        None,
        None,
        None,
    )
    .await;
    assert_eq!(fulfillment_email.status(), StatusCode::OK);
    assert_eq!(
        response_json(fulfillment_email).await["subject"],
        "Your KnitPrint order is on its way"
    );

    let completed = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/fulfillments"),
        Some(&owner_cookie),
        Some(json!({
            "carrier": "",
            "tracking_number": "",
            "tracking_url": "",
            "reason": "Final parcel collected",
            "lines": [{ "order_line_id": line_id, "quantity": 1 }]
        })),
        Some("order-fulfillment-0002"),
    )
    .await;
    assert_eq!(completed.status(), StatusCode::CREATED);
    let completed_body = response_json(completed).await;
    assert_eq!(completed_body["order_status"], "completed");
    assert_eq!(completed_body["fulfillment_status"], "fulfilled");
    assert_eq!(completed_body["lines"][0]["fulfilled_quantity"], 2);
    assert_eq!(completed_body["fulfillments"].as_array().unwrap().len(), 2);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action = 'order.fulfill' AND entity_id = $1",
    )
    .bind(order_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 2);

    let denied_refund = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/refunds"),
        Some(&reader_cookie),
        Some(json!({
            "mode": "partial",
            "lines": [{ "order_line_id": line_id, "quantity": 1 }],
            "restock": true,
            "reason": "Unauthorized return attempt",
            "internal_note": ""
        })),
        Some("order-refund-denied-0001"),
    )
    .await;
    assert_eq!(denied_refund.status(), StatusCode::FORBIDDEN);

    let partial_refund = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/refunds"),
        Some(&owner_cookie),
        Some(json!({
            "mode": "partial",
            "lines": [{ "order_line_id": line_id, "quantity": 1 }],
            "restock": true,
            "reason": "One item was returned",
            "internal_note": "Return inspected in the development test"
        })),
        Some("order-refund-partial-0001"),
    )
    .await;
    assert_eq!(partial_refund.status(), StatusCode::OK);
    let partial_refund_body = response_json(partial_refund).await;
    assert_eq!(partial_refund_body["payment_status"], "partially_refunded");
    assert_eq!(partial_refund_body["operations"]["refundable_minor"], 4991);
    assert_eq!(partial_refund_body["refunds"][0]["amount_minor"], 3200);
    assert_eq!(partial_refund_body["refunds"][0]["restock"], true);

    let refund_replay = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/refunds"),
        Some(&owner_cookie),
        Some(json!({
            "mode": "partial",
            "lines": [{ "order_line_id": line_id, "quantity": 1 }],
            "restock": true,
            "reason": "One item was returned",
            "internal_note": "Return inspected in the development test"
        })),
        Some("order-refund-partial-0001"),
    )
    .await;
    assert_eq!(refund_replay.status(), StatusCode::OK);
    assert_eq!(
        response_json(refund_replay).await["refunds"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let conflicting_refund = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/refunds"),
        Some(&owner_cookie),
        Some(json!({
            "mode": "full",
            "lines": [],
            "restock": false,
            "reason": "Conflicting retry",
            "internal_note": ""
        })),
        Some("order-refund-partial-0001"),
    )
    .await;
    assert_eq!(conflicting_refund.status(), StatusCode::CONFLICT);

    let final_refund = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{order_id}/refunds"),
        Some(&owner_cookie),
        Some(json!({
            "mode": "full",
            "lines": [],
            "restock": true,
            "reason": "Refund remaining paid balance",
            "internal_note": "Full refund after the partial return"
        })),
        Some("order-refund-full-0001"),
    )
    .await;
    assert_eq!(final_refund.status(), StatusCode::OK);
    let final_refund_body = response_json(final_refund).await;
    assert_eq!(final_refund_body["payment_status"], "refunded");
    assert_eq!(final_refund_body["order_status"], "completed");
    assert_eq!(final_refund_body["operations"]["can_refund"], false);
    assert_eq!(final_refund_body["refunds"].as_array().unwrap().len(), 2);
    let quantities: (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, committed_quantity FROM inventory_items WHERE variant_id = $1",
    )
    .bind(variant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quantities, (5, 0, 0));
    let refund_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action IN ('order.refund_requested', 'order.refund') AND entity_id = $1",
    )
    .bind(order_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(refund_audits, 4);

    let cancellable_cart = request(&router, "GET", "/api/cart", None, None, None).await;
    let cancellable_cookie = response_cookie(&cancellable_cart);
    assert_eq!(
        request(
            &router,
            "POST",
            "/api/cart/items",
            Some(&cancellable_cookie),
            Some(json!({ "variant_id": variant_id, "quantity": 1 })),
            Some("cancel-cart-add-0001"),
        )
        .await
        .status(),
        StatusCode::CREATED
    );
    assert_eq!(
        request(
            &router,
            "POST",
            "/api/cart/delivery",
            Some(&cancellable_cookie),
            Some(delivery_fixture()),
            Some("cancel-delivery-0001"),
        )
        .await
        .status(),
        StatusCode::OK
    );
    let cancellable_order = request(
        &router,
        "POST",
        "/api/orders",
        Some(&cancellable_cookie),
        Some(json!({ "payment_method": "manual" })),
        Some("cancel-checkout-0001"),
    )
    .await;
    let cancellable_order_id = response_json(cancellable_order).await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let cancelled = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{cancellable_order_id}/cancel"),
        Some(&owner_cookie),
        Some(json!({
            "reason": "Customer requested cancellation",
            "internal_note": "Confirmed before payment"
        })),
        Some("order-cancellation-0001"),
    )
    .await;
    assert_eq!(cancelled.status(), StatusCode::OK);
    let cancelled_body = response_json(cancelled).await;
    assert_eq!(cancelled_body["order_status"], "cancelled");
    assert_eq!(cancelled_body["payment_status"], "cancelled");
    assert_eq!(cancelled_body["operations"]["can_cancel"], false);
    let cancellation_replay = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{cancellable_order_id}/cancel"),
        Some(&owner_cookie),
        Some(json!({
            "reason": "Customer requested cancellation",
            "internal_note": "Confirmed before payment"
        })),
        Some("order-cancellation-0001"),
    )
    .await;
    assert_eq!(cancellation_replay.status(), StatusCode::OK);
    let cancellation_conflict = request(
        &router,
        "POST",
        &format!("/api/admin/orders/{cancellable_order_id}/cancel"),
        Some(&owner_cookie),
        Some(json!({
            "reason": "Different cancellation reason",
            "internal_note": ""
        })),
        Some("order-cancellation-0001"),
    )
    .await;
    assert_eq!(cancellation_conflict.status(), StatusCode::CONFLICT);
    let cancellation_audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action = 'order.cancel' AND entity_id = $1",
    )
    .bind(&cancellable_order_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cancellation_audits, 1);
    let quantities: (i64, i64, i64) = sqlx::query_as(
        "SELECT available_quantity, reserved_quantity, committed_quantity FROM inventory_items WHERE variant_id = $1",
    )
    .bind(variant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(quantities, (5, 0, 0));
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
    insert_owner(&pool).await;
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

    let owner_cookie = login(&router).await;
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
    assert_eq!(quantities, (7, 0, 0));

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
async fn concurrent_checkout_cannot_exceed_a_discount_usage_limit() {
    let Some(database_url) = env::var("DATABASE_URL").ok() else {
        eprintln!("skipping PostgreSQL integration test because DATABASE_URL is not set");
        return;
    };
    let schema = format!("discount_limit_test_{}", Uuid::new_v4().simple());
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
    let variant_id = insert_product(&pool).await;
    sqlx::query("INSERT INTO discounts (id,code,kind,fixed_amount_minor,currency,usage_limit) VALUES ($1,'ONLYONCE','fixed',500,'EUR',1)")
        .bind(Uuid::now_v7()).execute(&pool).await.unwrap();
    let router = app(AppState {
        database: Some(pool.clone()),
        manual_payments_enabled: true,
        ..AppState::default()
    });
    let first_cookie =
        prepare_discount_cart(&router, variant_id, "first", "first@example.com").await;
    let second_cookie =
        prepare_discount_cart(&router, variant_id, "second", "second@example.com").await;

    let first = request(
        &router,
        "POST",
        "/api/orders",
        Some(&first_cookie),
        Some(json!({ "payment_method": "manual" })),
        Some("discount-limit-checkout-first"),
    );
    let second = request(
        &router,
        "POST",
        "/api/orders",
        Some(&second_cookie),
        Some(json!({ "payment_method": "manual" })),
        Some("discount-limit-checkout-second"),
    );
    let (first, second) = tokio::join!(first, second);
    let mut statuses = [first.status(), second.status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM discount_usages), (SELECT count(*) FROM orders)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(counts, (1, 1));

    pool.close().await;
    sqlx::query(&format!(r#"DROP SCHEMA "{schema}" CASCADE"#))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}

async fn prepare_discount_cart(
    router: &axum::Router,
    variant_id: Uuid,
    suffix: &str,
    email: &str,
) -> String {
    let cart = request(router, "GET", "/api/cart", None, None, None).await;
    let cookie = response_cookie(&cart);
    assert_eq!(
        request(
            router,
            "POST",
            "/api/cart/items",
            Some(&cookie),
            Some(json!({ "variant_id": variant_id, "quantity": 1 })),
            Some(&format!("discount-limit-add-{suffix}")),
        )
        .await
        .status(),
        StatusCode::CREATED
    );
    let mut delivery = delivery_fixture();
    delivery["email"] = json!(email);
    assert_eq!(
        request(
            router,
            "POST",
            "/api/cart/delivery",
            Some(&cookie),
            Some(delivery),
            Some(&format!("discount-limit-delivery-{suffix}")),
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            router,
            "POST",
            "/api/cart/discount",
            Some(&cookie),
            Some(json!({ "code": "onlyonce" })),
            Some(&format!("discount-limit-apply-{suffix}")),
        )
        .await
        .status(),
        StatusCode::OK
    );
    cookie
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
