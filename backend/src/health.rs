use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{AppState, error::ErrorBody};

#[derive(Serialize, ToSchema)]
pub struct Health {
    status: &'static str,
    service: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "system",
    responses((status = 200, description = "The process is healthy", body = Health))
)]
pub async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        service: "knitprint-api",
    })
}

#[utoipa::path(
    get,
    path = "/api/ready",
    tag = "system",
    responses(
        (status = 200, description = "The API and database are ready", body = Health),
        (status = 503, description = "The database is unavailable", body = ErrorBody)
    )
)]
pub async fn ready(State(state): State<AppState>) -> Response {
    let Some(database) = state.database else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody::new(
                "database_unavailable",
                "The database connection is not configured.",
            )),
        )
            .into_response();
    };

    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&database)
        .await
    {
        Ok(_) => Json(Health {
            status: "ready",
            service: "knitprint-api",
        })
        .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody::new(
                "database_unavailable",
                "The database did not pass its readiness check.",
            )),
        )
            .into_response(),
    }
}
