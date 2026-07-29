use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    code: &'static str,
    message: &'static str,
}

impl ErrorBody {
    pub fn new(code: &'static str, message: &'static str) -> Self {
        Self {
            error: ErrorDetail { code, message },
        }
    }
}

pub async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody::new(
            "not_found",
            "The requested resource was not found.",
        )),
    )
        .into_response()
}
