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
    pub available_quantity: i64,
    pub low_stock: bool,
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
    pub categories: Vec<Category>,
}

#[derive(Clone, Serialize, ToSchema, FromRow)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct ProductMedia {
    pub id: Uuid,
    pub alt_text: String,
    pub position: i32,
    pub url: String,
    pub thumbnail_url: String,
    pub card_url: String,
    pub detail_url: String,
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

#[derive(Deserialize, ToSchema)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Deserialize, ToSchema)]
pub struct AssignCategoriesRequest {
    pub category_ids: Vec<Uuid>,
}

#[derive(Deserialize, IntoParams)]
pub struct ProductQuery {
    pub q: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize, IntoParams)]
pub struct PublicProductQuery {
    pub q: Option<String>,
    pub category: Option<String>,
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
    match product_rows(
        &pool,
        query.q.as_deref(),
        query.status.as_deref(),
        None,
        false,
    )
    .await
    {
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
    path = "/api/admin/categories",
    tag = "admin catalog",
    responses((status = 200, body = [Category]), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody))
)]
pub async fn category_list(State(state): State<AppState>, actor: AuthenticatedStaff) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_READ) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match sqlx::query_as::<_, Category>(
        "SELECT id, name, slug, description FROM categories ORDER BY name, id",
    )
    .fetch_all(&pool)
    .await
    {
        Ok(categories) => Json(categories).into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/categories",
    tag = "admin catalog",
    request_body = CreateCategoryRequest,
    responses((status = 201, body = Category), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 409, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn category_create(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Json(input): Json<CreateCategoryRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    if input.name.trim().is_empty()
        || input.name.trim().len() > 120
        || !valid_slug(input.slug.trim())
        || input.description.len() > 2_000
    {
        return invalid_input();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let category = Category {
        id: Uuid::now_v7(),
        name: input.name.trim().into(),
        slug: input.slug.trim().into(),
        description: input.description.trim().into(),
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let inserted =
        sqlx::query("INSERT INTO categories (id, name, slug, description) VALUES ($1, $2, $3, $4)")
            .bind(category.id)
            .bind(&category.name)
            .bind(&category.slug)
            .bind(&category.description)
            .execute(&mut *transaction)
            .await;
    match inserted {
        Ok(_) => {}
        Err(error) => return database_write_error(error),
    }
    if audit_entity(
        &mut transaction,
        actor.id,
        "category.create",
        "category",
        category.id,
        None,
    )
    .await
    .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    (StatusCode::CREATED, Json(category)).into_response()
}

#[utoipa::path(
    post,
    path = "/api/admin/products/{product_id}/variants",
    params(("product_id" = Uuid, Path)),
    tag = "admin catalog",
    request_body = CreateVariantRequest,
    responses((status = 201, body = Product), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn add_variant(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(product_id): Path<Uuid>,
    Json(input): Json<CreateVariantRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    if !valid_variant(&input) {
        return invalid_input();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let position: Option<i32> = match sqlx::query_scalar(
        r#"
        SELECT COALESCE(max(variant.position) + 1, 0)
        FROM products product
        LEFT JOIN product_variants variant ON variant.product_id = product.id
        WHERE product.id = $1
        GROUP BY product.id
        "#,
    )
    .bind(product_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(position) => position,
        Err(_) => return unavailable(),
    };
    let Some(position) = position else {
        return not_found();
    };
    let inserted = sqlx::query(
        r#"
        INSERT INTO product_variants (id, product_id, title, sku, price_minor, currency, option_values, position)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(product_id)
    .bind(input.title.trim())
    .bind(input.sku.trim())
    .bind(input.price_minor)
    .bind(input.currency.trim())
    .bind(input.option_values)
    .bind(position)
    .execute(&mut *transaction)
    .await;
    if let Err(error) = inserted {
        return database_write_error(error);
    }
    if audit(
        &mut transaction,
        actor.id,
        "product.variant_add",
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
    path = "/api/admin/products/{product_id}/categories",
    params(("product_id" = Uuid, Path)),
    tag = "admin catalog",
    request_body = AssignCategoriesRequest,
    responses((status = 200, body = Product), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn assign_categories(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(product_id): Path<Uuid>,
    Json(input): Json<AssignCategoriesRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    let category_ids: Vec<Uuid> = input
        .category_ids
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if category_ids.len() > 50 {
        return invalid_input();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let product_exists: bool =
        match sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM products WHERE id = $1)")
            .bind(product_id)
            .fetch_one(&pool)
            .await
        {
            Ok(exists) => exists,
            Err(_) => return unavailable(),
        };
    if !product_exists {
        return not_found();
    }
    let known: i64 = match sqlx::query_scalar("SELECT count(*) FROM categories WHERE id = ANY($1)")
        .bind(&category_ids)
        .fetch_one(&pool)
        .await
    {
        Ok(count) => count,
        Err(_) => return unavailable(),
    };
    if known != category_ids.len() as i64 {
        return invalid_input();
    }
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    if sqlx::query("DELETE FROM product_categories WHERE product_id = $1")
        .bind(product_id)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return unavailable();
    }
    for (position, category_id) in category_ids.iter().enumerate() {
        if sqlx::query("INSERT INTO product_categories (product_id, category_id, position) VALUES ($1, $2, $3)")
            .bind(product_id)
            .bind(category_id)
            .bind(position as i32)
            .execute(&mut *transaction)
            .await
            .is_err()
        {
            return unavailable();
        }
    }
    if audit(
        &mut transaction,
        actor.id,
        "product.categories_assign",
        product_id,
        None,
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
    match product_rows(
        &pool,
        query.q.as_deref(),
        Some("active"),
        query.category.as_deref(),
        true,
    )
    .await
    {
        Ok(rows) => products_response(&pool, rows).await,
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    get,
    path = "/api/categories",
    tag = "catalog",
    responses((status = 200, body = [Category]))
)]
pub async fn public_category_list(State(state): State<AppState>) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    match sqlx::query_as::<_, Category>(
        r#"
        SELECT category.id, category.name, category.slug, category.description
        FROM categories category
        WHERE EXISTS (
            SELECT 1
            FROM product_categories assignment
            JOIN products product ON product.id = assignment.product_id
            WHERE assignment.category_id = category.id AND product.status = 'active'
        )
        ORDER BY category.name, category.id
        "#,
    )
    .fetch_all(&pool)
    .await
    {
        Ok(categories) => Json(categories).into_response(),
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
    category: Option<&str>,
    public_order: bool,
) -> Result<Vec<ProductRow>, sqlx::Error> {
    let query = query.unwrap_or("").trim();
    let status = status.unwrap_or("").trim();
    let category = category.unwrap_or("").trim();
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
          AND (
            $3 = '' OR EXISTS (
              SELECT 1
              FROM product_categories assignment
              JOIN categories category ON category.id = assignment.category_id
              WHERE assignment.product_id = products.id AND category.slug = $3
            )
          )
        ORDER BY
            CASE WHEN $4 THEN published_at END DESC NULLS LAST,
            created_at DESC,
            id
        LIMIT 100
        "#,
    )
    .bind(status)
    .bind(query)
    .bind(category)
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
        SELECT
            variant.id,
            variant.title,
            variant.sku,
            variant.price_minor,
            variant.currency::text AS currency,
            variant.option_values,
            variant.position,
            inventory.available_quantity,
            inventory.available_quantity <= inventory.low_stock_threshold AS low_stock
        FROM product_variants variant
        JOIN inventory_items inventory ON inventory.variant_id = variant.id
        WHERE variant.product_id = $1
        ORDER BY variant.position, variant.id
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
            '/api/media/' || m.id::text || '/detail' AS url,
            '/api/media/' || m.id::text || '/thumbnail' AS thumbnail_url,
            '/api/media/' || m.id::text || '/card' AS card_url,
            '/api/media/' || m.id::text || '/detail' AS detail_url
        FROM product_media pm
        JOIN media_assets m ON m.id = pm.media_asset_id
        WHERE pm.product_id = $1
          AND m.status = 'ready'
          AND (SELECT count(*) FROM media_variants mv WHERE mv.media_asset_id = m.id) = 3
        ORDER BY pm.position, m.id
        "#,
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    let categories = sqlx::query_as::<_, Category>(
        r#"
        SELECT category.id, category.name, category.slug, category.description
        FROM product_categories assignment
        JOIN categories category ON category.id = assignment.category_id
        WHERE assignment.product_id = $1
        ORDER BY assignment.position, category.id
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
        categories,
    })
}

fn valid_product(input: &CreateProductRequest) -> bool {
    let slug = input.slug.trim();
    let currency_and_variants_valid = !input.variants.is_empty()
        && input.variants.len() <= 100
        && input.variants.iter().all(valid_variant);
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

fn valid_variant(variant: &CreateVariantRequest) -> bool {
    !variant.title.trim().is_empty()
        && variant.title.trim().len() <= 200
        && !variant.sku.trim().is_empty()
        && variant.sku.trim().len() <= 120
        && variant.price_minor >= 0
        && variant.currency.trim().len() == 3
        && variant
            .currency
            .trim()
            .chars()
            .all(|character| character.is_ascii_uppercase())
        && variant.option_values.is_object()
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
    audit_entity(transaction, actor, action, "product", entity_id, reason).await
}

async fn audit_entity(
    transaction: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(actor)
    .bind(action)
    .bind(entity_type)
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
