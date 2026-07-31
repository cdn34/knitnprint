pub mod auth;
pub mod catalog;
pub mod config;
pub mod error;
pub mod health;
pub mod media;
pub mod openapi;
pub mod staff;

use axum::{
    Json, Router,
    http::{HeaderName, Method},
    routing::get,
};
use sqlx::PgPool;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

#[derive(Clone, Default)]
pub struct AppState {
    pub database: Option<PgPool>,
    pub media_storage: Option<media::MediaStorage>,
    pub secure_cookies: bool,
}

pub fn app(state: AppState) -> Router {
    let request_id = HeaderName::from_static("x-request-id");

    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/ready", get(health::ready))
        .route("/api/admin/auth/login", axum::routing::post(auth::login))
        .route("/api/admin/auth/logout", axum::routing::post(auth::logout))
        .route("/api/admin/auth/me", get(auth::me))
        .route(
            "/api/admin/products",
            get(catalog::admin_list).post(catalog::create),
        )
        .route(
            "/api/admin/products/{product_id}",
            get(catalog::admin_detail),
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
            "/api/admin/media/uploads",
            axum::routing::post(media::initiate),
        )
        .route(
            "/api/admin/media/uploads/{media_id}/complete",
            axum::routing::post(media::complete),
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
        .route("/api/products", get(catalog::public_list))
        .route("/api/categories", get(catalog::public_category_list))
        .route("/api/products/{slug}", get(catalog::public_detail))
        .route("/api/media/{media_id}/{variant}", get(media::public_asset))
        .fallback(error::not_found)
        .with_state(state)
        .layer(PropagateRequestIdLayer::new(request_id.clone()))
        .layer(SetRequestIdLayer::new(request_id, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(Any),
        )
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
}
