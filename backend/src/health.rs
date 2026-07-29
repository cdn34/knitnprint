use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::{AppState, error::ErrorBody};

#[derive(Serialize)]
pub(crate) struct Health {
    status: &'static str,
    service: &'static str,
}

pub async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        service: "knitprint-api",
    })
}

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
