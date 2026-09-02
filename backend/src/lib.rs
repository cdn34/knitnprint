pub mod auth;
pub mod cancellations;
pub mod carts;
pub mod catalog;
pub mod config;
pub mod customer_auth;
pub mod customer_retention;
pub mod customers;
pub mod dashboard;
pub mod discounts;
pub mod email;
pub mod error;
pub mod fulfillment;
pub mod health;
pub mod inventory;
pub mod login_rate_limit;
pub mod media;
pub mod media_scanner;
pub mod notifications;
pub mod openapi;
pub mod orders;
pub mod packlink;
pub mod payments;
pub mod security;
pub mod settings;
pub mod staff;

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::{HeaderName, HeaderValue, Method, header},
    middleware,
    response::IntoResponse,
    routing::get,
};
use sqlx::PgPool;
use tower_http::{
    catch_panic::CatchPanicLayer,
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

#[derive(Clone, Default)]
pub struct AppState {
    pub database: Option<PgPool>,
    pub media_storage: Option<media::MediaStorage>,
    pub media_scanner: media_scanner::MediaScanner,
    pub email: email::EmailService,
    pub payments: payments::PaymentService,
    pub packlink: packlink::PacklinkService,
    pub trust_proxy_headers: bool,
    pub secure_cookies: bool,
    pub manual_payments_enabled: bool,
    pub security: security::SecurityPolicy,
}

pub fn app(state: AppState) -> Router {
    let request_id = HeaderName::from_static("x-request-id");
    let security = state.security.clone();
    let allowed_origins = security
        .allowed_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/ready", get(health::ready))
        .route("/api/admin/auth/login", axum::routing::post(auth::login))
        .route("/api/admin/auth/logout", axum::routing::post(auth::logout))
        .route("/api/admin/auth/me", get(auth::me))
        .route(
            "/api/account/register",
            axum::routing::post(customer_auth::register),
        )
        .route(
            "/api/account/login",
            axum::routing::post(customer_auth::login),
        )
        .route(
            "/api/account/logout",
            axum::routing::post(customer_auth::logout),
        )
        .route("/api/account/me", get(customer_auth::me))
        .route(
            "/api/development/emails/latest",
            get(email::development_latest),
        )
        .route(
            "/api/account/addresses",
            axum::routing::post(customer_auth::add_address),
        )
        .route(
            "/api/account/verification/request",
            axum::routing::post(customer_auth::request_verification),
        )
        .route(
            "/api/account/verification/confirm",
            axum::routing::post(customer_auth::confirm_verification),
        )
        .route(
            "/api/account/password/forgot",
            axum::routing::post(customer_auth::forgot_password),
        )
        .route(
            "/api/account/password/reset",
            axum::routing::post(customer_auth::reset_password),
        )
        .route(
            "/api/admin/products",
            get(catalog::admin_list).post(catalog::create),
        )
        .route(
            "/api/admin/products/{product_id}",
            get(catalog::admin_detail)
                .put(catalog::update)
                .delete(catalog::delete),
        )
        .route(
            "/api/admin/products/{product_id}/status",
            axum::routing::post(catalog::change_status),
        )
        .route(
            "/api/admin/products/{product_id}/variants",
            axum::routing::post(catalog::add_variant),
        )
        .route(
            "/api/admin/products/{product_id}/categories",
            axum::routing::post(catalog::assign_categories),
        )
        .route(
            "/api/admin/categories",
            get(catalog::category_list).post(catalog::category_create),
        )
        .route(
            "/api/admin/categories/order",
            axum::routing::put(catalog::category_reorder),
        )
        .route(
            "/api/admin/shipping-packages",
            get(catalog::shipping_package_list).post(catalog::shipping_package_create),
        )
        .route(
            "/api/admin/shipping-packages/{profile_id}",
            axum::routing::put(catalog::shipping_package_update)
                .delete(catalog::shipping_package_delete),
        )
        .route("/api/admin/inventory", get(inventory::list))
        .route("/api/admin/customers", get(customers::list))
        .route("/api/admin/customers/{customer_id}", get(customers::detail))
        .route(
            "/api/admin/customers/{customer_id}/orders",
            get(customers::order_history),
        )
        .route("/api/admin/orders", get(orders::admin_list))
        .route("/api/admin/dashboard", get(dashboard::get))
        .route(
            "/api/admin/discounts",
            get(discounts::list).post(discounts::create),
        )
        .route(
            "/api/admin/discounts/{discount_id}",
            axum::routing::put(discounts::update),
        )
        .route(
            "/api/admin/discounts/{discount_id}/status",
            axum::routing::post(discounts::change_status),
        )
        .route(
            "/api/admin/settings",
            get(settings::get).post(settings::update),
        )
        .route("/api/admin/orders/{order_id}", get(orders::admin_detail))
        .route(
            "/api/admin/orders/{order_id}/manual-payment",
            axum::routing::post(orders::record_manual_payment),
        )
        .route(
            "/api/admin/orders/{order_id}/fulfillments",
            axum::routing::post(fulfillment::create),
        )
        .route(
            "/api/admin/orders/{order_id}/cancel",
            axum::routing::post(cancellations::cancel),
        )
        .route(
            "/api/admin/orders/{order_id}/refunds",
            axum::routing::post(cancellations::refund),
        )
        .route(
            "/api/admin/inventory/{variant_id}/movements",
            get(inventory::movements),
        )
        .route(
            "/api/admin/inventory/{variant_id}/adjust",
            axum::routing::post(inventory::adjust),
        )
        .route(
            "/api/admin/media/uploads",
            axum::routing::post(media::initiate),
        )
        .route(
            "/api/admin/media/uploads/{media_id}/complete",
            axum::routing::post(media::complete),
        )
        .route(
            "/api/personalization/uploads",
            axum::routing::post(media::initiate_personalization),
        )
        .route(
            "/api/personalization/uploads/{media_id}/complete",
            axum::routing::post(media::complete_personalization),
        )
        .route(
            "/api/admin/personalization/media/{media_id}/{variant}",
            get(media::admin_personalization_asset),
        )
        .route(
            "/api/admin/order-product/media/{media_id}/{variant}",
            get(media::admin_order_product_asset),
        )
        .route("/api/admin/staff", get(staff::list).post(staff::create))
        .route(
            "/api/admin/staff/{staff_id}/disable",
            axum::routing::post(staff::disable),
        )
        .route(
            "/api/openapi.json",
            get(|| async { Json(openapi::document()) }),
        )
        .route("/api/payments/options", get(payments::options))
        .route(
            "/api/payments/stripe/webhook",
            axum::routing::post(payments::stripe_webhook),
        )
        .route("/api/products", get(catalog::public_list))
        .route("/api/orders", axum::routing::post(orders::create))
        .route("/api/orders/{order_id}", get(orders::customer_detail))
        .route(
            "/api/orders/{order_id}/payment",
            axum::routing::post(payments::start_checkout),
        )
        .route("/api/cart", get(carts::get))
        .route(
            "/api/cart/shipping-quotes",
            axum::routing::post(carts::refresh_shipping_quotes),
        )
        .route("/api/cart/items", axum::routing::post(carts::add_item))
        .route(
            "/api/cart/discount",
            axum::routing::post(carts::apply_discount).delete(carts::remove_discount),
        )
        .route(
            "/api/cart/shipping-method",
            axum::routing::post(carts::select_shipping_method),
        )
        .route(
            "/api/cart/items/{line_id}",
            axum::routing::patch(carts::update_item).delete(carts::remove_item),
        )
        .route(
            "/api/cart/delivery",
            axum::routing::post(carts::set_delivery),
        )
        .route(
            "/api/customers/guest",
            axum::routing::post(customers::create_guest),
        )
        .route("/api/categories", get(catalog::public_category_list))
        .route("/api/products/{slug}", get(catalog::public_detail))
        .route("/api/media/{media_id}/{variant}", get(media::public_asset))
        .fallback(error::not_found)
        .with_state(state)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(CatchPanicLayer::custom(|_| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(error::ErrorBody::new(
                    "internal_error",
                    "The request could not be completed.",
                )),
            )
                .into_response()
        }))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed_origins))
                .allow_credentials(true)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                ])
                .allow_headers([
                    header::CONTENT_TYPE,
                    HeaderName::from_static("idempotency-key"),
                ]),
        )
        .layer(middleware::from_fn_with_state(security, security::enforce))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::{AppState, app};

    #[tokio::test]
    async fn health_is_available_without_a_database() {
        let response = app(AppState::default())
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "knitprint-api");
    }

    #[tokio::test]
    async fn readiness_explains_a_missing_database() {
        let response = app(AppState::default())
            .oneshot(
                Request::builder()
                    .uri("/api/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), 2048).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "database_unavailable");
    }

    #[tokio::test]
    async fn unknown_routes_use_the_error_contract() {
        let response = app(AppState::default())
            .oneshot(
                Request::builder()
                    .uri("/api/unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), 2048).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "not_found");
    }

    #[tokio::test]
    async fn openapi_contract_is_served() {
        let response = app(AppState::default())
            .oneshot(
                Request::builder()
                    .uri("/api/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 128 * 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["info"]["title"], "KnitPrint API");
        assert!(json["paths"]["/api/health"].is_object());
    }

    #[tokio::test]
    async fn responses_include_browser_security_headers() {
        let response = app(AppState::default())
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert_eq!(response.headers()["x-frame-options"], "DENY");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
        assert!(response.headers().contains_key("content-security-policy"));
        assert!(!response.headers().contains_key("strict-transport-security"));
    }

    #[tokio::test]
    async fn unsafe_cross_origin_requests_are_rejected_before_handlers() {
        let response = app(AppState::default())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/admin/auth/login")
                    .header("origin", "https://attacker.example")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"email":"owner@example.com","password":"password"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(response.into_body(), 2048).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"]["code"], "cross_origin_request_rejected");
    }

    #[tokio::test]
    async fn configured_origins_receive_credentialed_cors_headers() {
        let response = app(AppState::default())
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/account/login")
                    .header("origin", "http://localhost:3000")
                    .header("access-control-request-method", "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["access-control-allow-origin"],
            "http://localhost:3000"
        );
        assert_eq!(
            response.headers()["access-control-allow-credentials"],
            "true"
        );
    }

    #[tokio::test]
    async fn production_responses_require_transport_security() {
        let mut state = AppState::default();
        state.security.production = true;
        let response = app(state)
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.headers()["strict-transport-security"],
            "max-age=31536000; includeSubDomains"
        );
    }

    #[tokio::test]
    async fn oversized_json_requests_are_rejected() {
        let response = app(AppState::default())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/account/login")
                    .header("origin", "http://localhost:3000")
                    .header("content-type", "application/json")
                    .body(Body::from(vec![b'a'; 1024 * 1024 + 1]))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
