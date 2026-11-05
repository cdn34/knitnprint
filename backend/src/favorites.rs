use axum::{
    Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::{AppState, catalog, customer_auth::AuthenticatedCustomer, error::ErrorBody};

#[utoipa::path(
    get,
    path = "/api/account/favorites",
    tag = "customer account",
    responses(
        (status = 200, body = [catalog::Product]),
        (status = 401, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn list(State(state): State<AppState>, customer: AuthenticatedCustomer) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    let ids = match sqlx::query_scalar::<_, Uuid>(
        "SELECT product_id FROM customer_favorites WHERE customer_id = $1 ORDER BY created_at DESC, product_id DESC",
    )
    .bind(customer.id)
    .fetch_all(&pool)
    .await
    {
        Ok(ids) => ids,
        Err(_) => return unavailable(),
    };
    match catalog::public_products_by_ids(&pool, &ids).await {
        Ok(products) => (
            [(header::CACHE_CONTROL, "no-store, private")],
            Json(products),
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    put,
    path = "/api/account/favorites/{product_id}",
    tag = "customer account",
    params(("product_id" = Uuid, Path)),
    responses(
        (status = 204, description = "Product saved, including an already saved product"),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn save(
    State(state): State<AppState>,
    customer: AuthenticatedCustomer,
    Path(product_id): Path<Uuid>,
) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    match sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO customer_favorites (customer_id, product_id)
        SELECT $1, id FROM products WHERE id = $2 AND status = 'active'
        ON CONFLICT (customer_id, product_id) DO UPDATE SET product_id = EXCLUDED.product_id
        RETURNING product_id"#,
    )
    .bind(customer.id)
    .bind(product_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody::new(
                "favorite_product_unavailable",
                "This product is no longer available.",
            )),
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    delete,
    path = "/api/account/favorites/{product_id}",
    tag = "customer account",
    params(("product_id" = Uuid, Path)),
    responses(
        (status = 204, description = "Product removed, including an already removed product"),
        (status = 401, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    customer: AuthenticatedCustomer,
    Path(product_id): Path<Uuid>,
) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    match sqlx::query("DELETE FROM customer_favorites WHERE customer_id = $1 AND product_id = $2")
        .bind(customer.id)
        .bind(product_id)
        .execute(&pool)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => unavailable(),
    }
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody::new(
            "favorites_unavailable",
            "Favorites are temporarily unavailable. Please try again.",
        )),
    )
        .into_response()
}
