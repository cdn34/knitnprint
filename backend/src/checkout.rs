use crate::error::ErrorBody;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{carts::CartAddress, delivery_rates::DeliveryQuote};

/// Compare the delivery and prices reviewed by a customer with checkout's
/// freshly calculated values. Amounts are always calculated by the server.
pub(crate) fn quote_fingerprint(
    cart_id: Uuid,
    currency: &str,
    address: &CartAddress,
    shipping: &DeliveryQuote,
    amounts: [i64; 4],
) -> String {
    let snapshot = serde_json::to_vec(&(
        "checkout-quote-v1",
        cart_id,
        currency,
        address,
        shipping,
        amounts,
    ))
    .expect("checkout quote contains only serializable strings and integers");
    hex::encode(Sha256::digest(snapshot))
}

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
