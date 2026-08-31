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
const PERSONALIZATION_FONTS: &[&str] = &[
    "Roboto",
    "Montserrat",
    "Playfair Display",
    "Dancing Script",
    "Pacifico",
];

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
    pub personalization: PersonalizationConfig,
}

#[derive(Clone, Serialize, Deserialize, ToSchema, FromRow)]
pub struct PersonalizationConfig {
    pub mode: String,
    pub preview_media_id: Option<Uuid>,
    pub area_x: i32,
    pub area_y: i32,
    pub area_width: i32,
    pub area_height: i32,
    pub text_area_x: i32,
    pub text_area_y: i32,
    pub text_area_width: i32,
    pub text_area_height: i32,
    pub text_max_characters: i32,
    pub text_min_size: i32,
    pub text_max_size: i32,
    pub allowed_fonts: Value,
    pub allowed_colors: Value,
}

impl Default for PersonalizationConfig {
    fn default() -> Self {
        Self {
            mode: "none".into(),
            preview_media_id: None,
            area_x: 2500,
            area_y: 2500,
            area_width: 5000,
            area_height: 5000,
            text_area_x: 2500,
            text_area_y: 6500,
            text_area_width: 5000,
            text_area_height: 2000,
            text_max_characters: 35,
            text_min_size: 12,
            text_max_size: 72,
            allowed_fonts: serde_json::json!([
                "Roboto",
                "Montserrat",
                "Playfair Display",
                "Dancing Script",
                "Pacifico"
            ]),
            allowed_colors: serde_json::json!([
                "#111111", "#ffffff", "#9c5263", "#1f4f78", "#b3232f"
            ]),
        }
    }
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
    #[serde(default)]
    pub personalization: PersonalizationConfig,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateVariantRequest {
    pub title: String,
    pub sku: String,
    pub price_minor: i64,
    pub currency: String,
    #[serde(default = "empty_object")]
    pub option_values: Value,
    #[serde(default)]
    pub available_quantity: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub search_keywords: String,
    pub sku: String,
    pub price_minor: i64,
    pub currency: String,
    pub available_quantity: i64,
    #[serde(default)]
    pub personalization: PersonalizationConfig,
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
        let variant_id = Uuid::now_v7();
        if let Err(error) = sqlx::query(
            r#"
            INSERT INTO product_variants (
                id, product_id, title, sku, price_minor, currency, option_values, position
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(variant_id)
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
        if variant.available_quantity > 0 {
            if sqlx::query("UPDATE inventory_items SET available_quantity = $2 WHERE variant_id = $1")
                .bind(variant_id)
                .bind(variant.available_quantity)
                .execute(&mut *transaction)
                .await
                .is_err()
                || sqlx::query("INSERT INTO inventory_movements (id, variant_id, actor_staff_user_id, movement_type, quantity_delta, resulting_available_quantity, reason) VALUES ($1, $2, $3, 'adjustment', $4, $4, 'Initial product stock')")
                    .bind(Uuid::now_v7()).bind(variant_id).bind(actor.id).bind(variant.available_quantity)
                    .execute(&mut *transaction).await.is_err()
            {
                return unavailable();
            }
        }
    }
    if upsert_personalization(&mut transaction, product_id, &input.personalization)
        .await
        .is_err()
    {
        return unavailable();
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
    put,
    path = "/api/admin/products/{product_id}",
    params(("product_id" = Uuid, Path)),
    tag = "admin catalog",
    request_body = UpdateProductRequest,
    responses((status = 200, body = Product), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody), (status = 422, body = ErrorBody))
)]
pub async fn update(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(product_id): Path<Uuid>,
    Json(input): Json<UpdateProductRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    let variant = CreateVariantRequest {
        title: "Default".into(),
        sku: input.sku.clone(),
        price_minor: input.price_minor,
        currency: input.currency.clone(),
        option_values: empty_object(),
        available_quantity: input.available_quantity,
    };
    if input.title.trim().is_empty()
        || input.title.trim().len() > 200
        || !valid_slug(input.slug.trim())
        || input.description.len() > 50_000
        || input.search_keywords.len() > 2_000
        || !valid_variant(&variant)
        || !valid_personalization(&input.personalization)
    {
        return invalid_input();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let base = sqlx::query_as::<_, (Uuid, i64)>("SELECT variant.id, inventory.available_quantity FROM product_variants variant JOIN inventory_items inventory ON inventory.variant_id=variant.id WHERE variant.product_id=$1 ORDER BY variant.position, variant.id LIMIT 1 FOR UPDATE OF inventory")
        .bind(product_id).fetch_optional(&mut *tx).await;
    let Some((variant_id, old_quantity)) = (match base {
        Ok(value) => value,
        Err(_) => return unavailable(),
    }) else {
        return not_found();
    };
    if let Err(error) = sqlx::query("UPDATE products SET title=$2, slug=$3, description=$4, search_keywords=$5, updated_at=now() WHERE id=$1")
        .bind(product_id).bind(input.title.trim()).bind(input.slug.trim()).bind(input.description.trim()).bind(input.search_keywords.trim()).execute(&mut *tx).await { return database_write_error(error); }
    if let Err(error) = sqlx::query("UPDATE product_variants SET sku=$2, price_minor=$3, currency=$4, updated_at=now() WHERE id=$1")
        .bind(variant_id).bind(input.sku.trim()).bind(input.price_minor).bind(input.currency.trim()).execute(&mut *tx).await { return database_write_error(error); }
    if old_quantity != input.available_quantity {
        let delta = input.available_quantity - old_quantity;
        if sqlx::query("UPDATE inventory_items SET available_quantity=$2, updated_at=now() WHERE variant_id=$1").bind(variant_id).bind(input.available_quantity).execute(&mut *tx).await.is_err()
            || sqlx::query("INSERT INTO inventory_movements (id, variant_id, actor_staff_user_id, movement_type, quantity_delta, resulting_available_quantity, reason) VALUES ($1,$2,$3,'adjustment',$4,$5,'Product editor stock correction')")
                .bind(Uuid::now_v7()).bind(variant_id).bind(actor.id).bind(delta).bind(input.available_quantity).execute(&mut *tx).await.is_err() { return unavailable(); }
    }
    if upsert_personalization(&mut tx, product_id, &input.personalization)
        .await
        .is_err()
    {
        return unavailable();
    }
    if audit(&mut tx, actor.id, "product.update", product_id, None)
        .await
        .is_err()
        || tx.commit().await.is_err()
    {
        return unavailable();
    }
    product_by_id(&pool, product_id).await
}

#[utoipa::path(delete, path = "/api/admin/products/{product_id}", params(("product_id" = Uuid, Path)), tag = "admin catalog", responses((status = 204), (status = 404, body = ErrorBody), (status = 409, body = ErrorBody)))]
pub async fn delete(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(product_id): Path<Uuid>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let sold = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM order_lines WHERE product_id=$1)",
    )
    .bind(product_id)
    .fetch_one(&pool)
    .await;
    match sold {
        Ok(true) => {
            return (
                StatusCode::CONFLICT,
                Json(ErrorBody::new(
                    "product_has_sales",
                    "Products with sales history cannot be deleted. Archive this product instead.",
                )),
            )
                .into_response();
        }
        Err(_) => return unavailable(),
        _ => {}
    }
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    if audit(&mut tx, actor.id, "product.delete", product_id, None)
        .await
        .is_err()
    {
        return unavailable();
    }
    match sqlx::query("DELETE FROM products WHERE id=$1")
        .bind(product_id)
        .execute(&mut *tx)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => return not_found(),
        Ok(_) => {}
        Err(_) => return unavailable(),
    }
    if tx.commit().await.is_err() {
        return unavailable();
    }
    StatusCode::NO_CONTENT.into_response()
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
        Ok(rows) => public_products_response(&pool, rows).await,
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
        Ok(Some(row)) => public_product_response(&pool, row).await,
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
            slug ILIKE '%' || $2 || '%' OR
            EXISTS (
              SELECT 1 FROM product_variants variant
              WHERE variant.product_id = products.id
                AND variant.sku ILIKE '%' || $2 || '%'
            )
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

async fn public_products_response(pool: &PgPool, rows: Vec<ProductRow>) -> Response {
    let mut products = Vec::with_capacity(rows.len());
    for row in rows {
        match hydrate_product(pool, row).await {
            Ok(mut product) => {
                product.search_keywords.clear();
                products.push(product);
            }
            Err(_) => return unavailable(),
        }
    }
    Json(products).into_response()
}

async fn public_product_response(pool: &PgPool, row: ProductRow) -> Response {
    match hydrate_product(pool, row).await {
        Ok(mut product) => {
            product.search_keywords.clear();
            Json(product).into_response()
        }
        Err(_) => unavailable(),
    }
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
    let personalization = sqlx::query_as::<_, PersonalizationConfig>(
        r#"
        SELECT mode, preview_media_asset_id AS preview_media_id,
               area_x, area_y, area_width, area_height,
               text_area_x, text_area_y, text_area_width, text_area_height,
               text_max_characters, text_min_size, text_max_size,
               allowed_fonts, allowed_colors
        FROM product_personalization WHERE product_id = $1
        "#,
    )
    .bind(row.id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_default();
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
        personalization,
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
        && valid_personalization(&input.personalization)
}

fn valid_personalization(config: &PersonalizationConfig) -> bool {
    matches!(
        config.mode.as_str(),
        "none" | "photo" | "text" | "photo_text"
    ) && config.area_x >= 0
        && config.area_y >= 0
        && config.area_width >= 100
        && config.area_height >= 100
        && config.area_x + config.area_width <= 10_000
        && config.area_y + config.area_height <= 10_000
        && config.text_area_x >= 0
        && config.text_area_y >= 0
        && config.text_area_width >= 100
        && config.text_area_height >= 100
        && config.text_area_x + config.text_area_width <= 10_000
        && config.text_area_y + config.text_area_height <= 10_000
        && (1..=500).contains(&config.text_max_characters)
        && (8..=200).contains(&config.text_min_size)
        && (config.text_min_size..=300).contains(&config.text_max_size)
        && valid_font_options(&config.allowed_fonts)
        && valid_color_options(&config.allowed_colors)
}

fn valid_font_options(value: &Value) -> bool {
    value.as_array().is_some_and(|items| {
        !items.is_empty()
            && items.len() <= PERSONALIZATION_FONTS.len()
            && items.iter().all(|item| {
                item.as_str()
                    .is_some_and(|font| PERSONALIZATION_FONTS.contains(&font))
            })
    })
}

fn valid_color_options(value: &Value) -> bool {
    value.as_array().is_some_and(|items| {
        !items.is_empty()
            && items.len() <= 30
            && items.iter().all(|item| {
                item.as_str().is_some_and(|color| {
                    color.len() == 7
                        && color.starts_with('#')
                        && color[1..]
                            .chars()
                            .all(|character| character.is_ascii_hexdigit())
                })
            })
    })
}

async fn upsert_personalization(
    tx: &mut Transaction<'_, Postgres>,
    product_id: Uuid,
    config: &PersonalizationConfig,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO product_personalization (
            product_id, mode, preview_media_asset_id, area_x, area_y, area_width, area_height,
            text_area_x, text_area_y, text_area_width, text_area_height,
            text_max_characters, text_min_size, text_max_size, allowed_fonts, allowed_colors
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
        ON CONFLICT (product_id) DO UPDATE SET
            mode=EXCLUDED.mode, preview_media_asset_id=EXCLUDED.preview_media_asset_id,
            area_x=EXCLUDED.area_x, area_y=EXCLUDED.area_y,
            area_width=EXCLUDED.area_width, area_height=EXCLUDED.area_height,
            text_area_x=EXCLUDED.text_area_x, text_area_y=EXCLUDED.text_area_y,
            text_area_width=EXCLUDED.text_area_width, text_area_height=EXCLUDED.text_area_height,
            text_max_characters=EXCLUDED.text_max_characters,
            text_min_size=EXCLUDED.text_min_size, text_max_size=EXCLUDED.text_max_size,
            allowed_fonts=EXCLUDED.allowed_fonts, allowed_colors=EXCLUDED.allowed_colors,
            updated_at=now()
        "#,
    )
    .bind(product_id)
    .bind(&config.mode)
    .bind(config.preview_media_id)
    .bind(config.area_x)
    .bind(config.area_y)
    .bind(config.area_width)
    .bind(config.area_height)
    .bind(config.text_area_x)
    .bind(config.text_area_y)
    .bind(config.text_area_width)
    .bind(config.text_area_height)
    .bind(config.text_max_characters)
    .bind(config.text_min_size)
    .bind(config.text_max_size)
    .bind(&config.allowed_fonts)
    .bind(&config.allowed_colors)
    .execute(&mut **tx)
    .await?;
    Ok(())
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
        && variant.available_quantity >= 0
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
