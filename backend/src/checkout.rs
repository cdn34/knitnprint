use crate::error::ErrorBody;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Orders currently use zero delivery charges until delivery pricing is configured.
pub const SUSPENDED: bool = false;

pub fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody::new(
            "checkout_suspended",
            "Checkout is temporarily suspended while delivery pricing is updated.",
        )),
    )
        .into_response()
}
