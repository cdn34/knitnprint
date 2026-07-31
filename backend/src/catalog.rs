use std::collections::BTreeSet;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
};

const CATALOG_READ: &str = "catalog.read";
const CATALOG_WRITE: &str = "catalog.write";

#[derive(Clone, Serialize, ToSchema, FromRow)]
pub struct Variant {
    pub id: Uuid,
    pub title: String,
    pub sku: String,
    pub price_minor: i64,
    pub currency: String,
    pub option_values: Value,
    pub position: i32,
}

#[derive(Serialize, ToSchema)]
pub struct Product {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub status: String,
    pub search_keywords: String,
    pub variants: Vec<Variant>,
    pub media: Vec<ProductMedia>,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct ProductMedia {
    pub id: Uuid,
    pub alt_text: String,
    pub position: i32,
    pub url: String,
}

#[derive(FromRow)]
struct ProductRow {
    id: Uuid,
    title: String,
    slug: String,
    description: String,
    status: String,
    search_keywords: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub search_keywords: String,
    pub variants: Vec<CreateVariantRequest>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateVariantRequest {
    pub title: String,
    pub sku: String,
    pub price_minor: i64,
    pub currency: String,
    #[serde(default = "empty_object")]
    pub option_values: Value,
}

#[derive(Deserialize, ToSchema)]
pub struct ChangeProductStatusRequest {
    pub status: String,
}

#[derive(Deserialize, IntoParams)]
pub struct ProductQuery {
    pub q: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize, IntoParams)]
pub struct PublicProductQuery {
    pub q: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/admin/products",
    params(ProductQuery),
    tag = "admin catalog",
    responses(
        (status = 200, body = [Product]),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody)
    )
)]
pub async fn admin_list(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Query(query): Query<ProductQuery>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_READ) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match product_rows(&pool, query.q.as_deref(), query.status.as_deref(), false).await {
        Ok(rows) => products_response(&pool, rows).await,
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/products/{product_id}",
    params(("product_id" = Uuid, Path)),
    tag = "admin catalog",
    responses(
        (status = 200, body = Product),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody)
    )
)]
pub async fn admin_detail(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(product_id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_READ) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    product_by_id(&pool, product_id).await
}

#[utoipa::path(
    post,
    path = "/api/admin/products",
    tag = "admin catalog",
    request_body = CreateProductRequest,
    responses(
        (status = 201, body = Product),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Json(input): Json<CreateProductRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    if !valid_product(&input) {
        return invalid_input();
    }
    let product_id = Uuid::now_v7();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if let Err(error) = sqlx::query(
        r#"
        INSERT INTO products (id, title, slug, description, search_keywords)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(product_id)
    .bind(input.title.trim())
    .bind(input.slug.trim())
    .bind(input.description.trim())
    .bind(input.search_keywords.trim())
    .execute(&mut *transaction)
    .await
    {
        return database_write_error(error);
    }
    for (position, variant) in input.variants.iter().enumerate() {
        if let Err(error) = sqlx::query(
            r#"
            INSERT INTO product_variants (
                id, product_id, title, sku, price_minor, currency, option_values, position
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(product_id)
        .bind(variant.title.trim())
        .bind(variant.sku.trim())
        .bind(variant.price_minor)
        .bind(variant.currency.trim())
        .bind(&variant.option_values)
        .bind(position as i32)
        .execute(&mut *transaction)
        .await
        {
            return database_write_error(error);
        }
    }
    if audit(
        &mut transaction,
        actor.id,
        "product.create",
        product_id,
        None,
    )
    .await
    .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    let mut response = product_by_id(&pool, product_id).await;
    *response.status_mut() = StatusCode::CREATED;
    response
}

#[utoipa::path(
    post,
    path = "/api/admin/products/{product_id}/status",
    params(("product_id" = Uuid, Path)),
    tag = "admin catalog",
    request_body = ChangeProductStatusRequest,
    responses(
        (status = 200, body = Product),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 422, body = ErrorBody)
    )
)]
pub async fn change_status(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(product_id): Path<Uuid>,
    Json(input): Json<ChangeProductStatusRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    if !matches!(input.status.as_str(), "draft" | "active" | "archived") {
        return invalid_input();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let changed = sqlx::query(
        r#"
        UPDATE products
        SET status = $2,
            published_at = CASE
                WHEN $2 = 'active' THEN COALESCE(published_at, now())
                ELSE published_at
            END,
            updated_at = now()
        WHERE id = $1
        "#,
    )
    .bind(product_id)
    .bind(&input.status)
    .execute(&mut *transaction)
    .await;
    match changed {
        Ok(result) if result.rows_affected() == 0 => return not_found(),
        Ok(_) => {}
        Err(_) => return unavailable(),
    }
    if audit(
        &mut transaction,
        actor.id,
        "product.status_change",
        product_id,
        Some(&input.status),
    )
    .await
    .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    product_by_id(&pool, product_id).await
}

#[utoipa::path(
    get,
    path = "/api/products",
    params(PublicProductQuery),
    tag = "catalog",
    responses((status = 200, body = [Product]))
)]
pub async fn public_list(
    State(state): State<AppState>,
    Query(query): Query<PublicProductQuery>,
) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    match product_rows(&pool, query.q.as_deref(), Some("active"), true).await {
        Ok(rows) => products_response(&pool, rows).await,
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    get,
    path = "/api/products/{slug}",
    params(("slug" = String, Path)),
    tag = "catalog",
    responses((status = 200, body = Product), (status = 404, body = ErrorBody))
)]
pub async fn public_detail(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    let row = sqlx::query_as::<_, ProductRow>(
        r#"
        SELECT id, title, slug, description, status, search_keywords
        FROM products
        WHERE slug = $1 AND status = 'active'
        "#,
    )
    .bind(slug)
    .fetch_optional(&pool)
    .await;
    match row {
        Ok(Some(row)) => product_response(&pool, row).await,
        Ok(None) => not_found(),
        Err(_) => unavailable(),
    }
}

async fn product_rows(
    pool: &PgPool,
    query: Option<&str>,
    status: Option<&str>,
    public_order: bool,
) -> Result<Vec<ProductRow>, sqlx::Error> {
    let query = query.unwrap_or("").trim();
    let status = status.unwrap_or("").trim();
    sqlx::query_as(
        r#"
        SELECT id, title, slug, description, status, search_keywords
        FROM products
        WHERE ($1 = '' OR status = $1)
          AND (
            $2 = '' OR
            search_document @@ websearch_to_tsquery('simple', $2) OR
            slug ILIKE '%' || $2 || '%'
          )
        ORDER BY
            CASE WHEN $3 THEN published_at END DESC NULLS LAST,
            created_at DESC,
            id
        LIMIT 100
        "#,
    )
    .bind(status)
    .bind(query)
    .bind(public_order)
    .fetch_all(pool)
    .await
}

async fn products_response(pool: &PgPool, rows: Vec<ProductRow>) -> Response {
    let mut products = Vec::with_capacity(rows.len());
    for row in rows {
        match hydrate_product(pool, row).await {
            Ok(product) => products.push(product),
            Err(_) => return unavailable(),
        }
    }
    Json(products).into_response()
}

async fn product_by_id(pool: &PgPool, id: Uuid) -> Response {
    match sqlx::query_as::<_, ProductRow>(
        "SELECT id, title, slug, description, status, search_keywords FROM products WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(row)) => product_response(pool, row).await,
        Ok(None) => not_found(),
        Err(_) => unavailable(),
    }
}

async fn product_response(pool: &PgPool, row: ProductRow) -> Response {
    match hydrate_product(pool, row).await {
        Ok(product) => Json(product).into_response(),
        Err(_) => unavailable(),
    }
}

async fn hydrate_product(pool: &PgPool, row: ProductRow) -> Result<Product, sqlx::Error> {
    let variants = sqlx::query_as::<_, Variant>(
        r#"
        SELECT id, title, sku, price_minor, currency::text AS currency, option_values, position
        FROM product_variants
        WHERE product_id = $1
        ORDER BY position, id
        "#,
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    let media = sqlx::query_as::<_, ProductMedia>(
        r#"
        SELECT
            m.id,
            pm.alt_text,
            pm.position,
            '/api/media/' || m.id::text AS url
        FROM product_media pm
        JOIN media_assets m ON m.id = pm.media_asset_id
        WHERE pm.product_id = $1 AND m.status = 'ready'
        ORDER BY pm.position, m.id
        "#,
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    Ok(Product {
        id: row.id,
        title: row.title,
        slug: row.slug,
        description: row.description,
        status: row.status,
        search_keywords: row.search_keywords,
        variants,
        media,
    })
}

fn valid_product(input: &CreateProductRequest) -> bool {
    let slug = input.slug.trim();
    let currency_and_variants_valid = !input.variants.is_empty()
        && input.variants.len() <= 100
        && input.variants.iter().all(|variant| {
            !variant.title.trim().is_empty()
                && !variant.sku.trim().is_empty()
                && variant.price_minor >= 0
                && variant.currency.trim().len() == 3
                && variant
                    .currency
                    .trim()
                    .chars()
                    .all(|character| character.is_ascii_uppercase())
                && variant.option_values.is_object()
        });
    let unique_skus = input
        .variants
        .iter()
        .map(|variant| variant.sku.trim())
        .collect::<BTreeSet<_>>()
        .len()
        == input.variants.len();
    !input.title.trim().is_empty()
        && input.title.trim().len() <= 200
        && valid_slug(slug)
        && input.description.len() <= 50_000
        && input.search_keywords.len() <= 2_000
        && currency_and_variants_valid
        && unique_skus
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 200
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

async fn audit(
    transaction: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    action: &str,
    entity_id: Uuid,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason)
        VALUES ($1, $2, 'product', $3, $4)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(entity_id.to_string())
    .bind(reason)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn empty_object() -> Value {
    serde_json::json!({})
}

fn database_write_error(error: sqlx::Error) -> Response {
    if error
        .as_database_error()
        .and_then(|database| database.code())
        .as_deref()
        == Some("23505")
    {
        (
            StatusCode::CONFLICT,
            Json(ErrorBody::new(
                "catalog_conflict",
                "A product slug or variant SKU already exists.",
            )),
        )
            .into_response()
    } else {
        unavailable()
    }
}

fn invalid_input() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorBody::new(
            "invalid_product",
            "The product or variant details are invalid.",
        )),
    )
        .into_response()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody::new(
            "product_not_found",
            "The product was not found.",
        )),
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody::new(
            "database_unavailable",
            "The catalog is temporarily unavailable.",
        )),
    )
        .into_response()
}
