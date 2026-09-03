use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
};

const CATALOG_READ: &str = "catalog.read";
const CATALOG_WRITE: &str = "catalog.write";

#[derive(Deserialize, ToSchema)]
pub struct CreateProductFeedbackRequest {
    pub display_name: String,
    pub rating: i16,
    pub comment: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct ProductFeedback {
    pub id: Uuid,
    pub display_name: String,
    pub rating: i16,
    pub comment: String,
    pub created_at: String,
    pub store_reply: Option<String>,
    pub replied_at: Option<String>,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct RatingCount {
    pub rating: i16,
    pub count: i64,
}

#[derive(FromRow)]
struct FeedbackAggregate {
    average_rating: Option<f64>,
    total_reviews: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ProductFeedbackSummary {
    pub average_rating: Option<f64>,
    pub total_reviews: i64,
    pub rating_counts: Vec<RatingCount>,
    pub reviews: Vec<ProductFeedback>,
}

#[derive(Serialize, ToSchema)]
pub struct SubmittedProductFeedback {
    pub id: Uuid,
    pub status: String,
}

#[derive(Serialize, ToSchema, FromRow)]
pub struct AdminProductFeedback {
    pub id: Uuid,
    pub product_id: Uuid,
    pub product_title: String,
    pub product_slug: String,
    pub display_name: String,
    pub rating: i16,
    pub comment: String,
    pub status: String,
    pub created_at: String,
    pub moderated_at: Option<String>,
    pub moderated_by_name: Option<String>,
    pub store_reply: Option<String>,
    pub replied_at: Option<String>,
    pub replied_by_name: Option<String>,
}

#[derive(Deserialize)]
pub struct AdminFeedbackQuery {
    pub status: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ModerateProductFeedbackRequest {
    pub status: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ReplyToProductFeedbackRequest {
    pub reply: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/products/{slug}/feedback",
    tag = "product feedback",
    params(("slug" = String, Path)),
    responses(
        (status = 200, body = ProductFeedbackSummary),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn public_list(State(state): State<AppState>, Path(slug): Path<String>) -> Response {
    let Some(pool) = state.database else {
        return unavailable();
    };
    let product_id = match active_product_id(&pool, &slug).await {
        Ok(Some(id)) => id,
        Ok(None) => return not_found(),
        Err(_) => return unavailable(),
    };
    let reviews = match sqlx::query_as::<_, ProductFeedback>(
        r#"
        SELECT id, display_name, rating, comment,
            to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
            store_reply,
            CASE WHEN replied_at IS NULL THEN NULL ELSE
                to_char(replied_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') END AS replied_at
        FROM product_feedback
        WHERE product_id = $1 AND status = 'approved'
        ORDER BY created_at DESC, id DESC
        LIMIT 100
        "#,
    )
    .bind(product_id)
    .fetch_all(&pool)
    .await
    {
        Ok(reviews) => reviews,
        Err(_) => return unavailable(),
    };
    let aggregate = match sqlx::query_as::<_, FeedbackAggregate>(
        "SELECT avg(rating::float8) AS average_rating,count(*) AS total_reviews FROM product_feedback WHERE product_id=$1 AND status='approved'",
    )
    .bind(product_id)
    .fetch_one(&pool)
    .await
    {
        Ok(aggregate) => aggregate,
        Err(_) => return unavailable(),
    };
    let counts = match sqlx::query_as::<_, RatingCount>(
        "SELECT rating,count(*) AS count FROM product_feedback WHERE product_id=$1 AND status='approved' GROUP BY rating",
    )
    .bind(product_id)
    .fetch_all(&pool)
    .await
    {
        Ok(counts) => counts,
        Err(_) => return unavailable(),
    };
    let rating_counts = (1_i16..=5)
        .rev()
        .map(|rating| RatingCount {
            rating,
            count: counts
                .iter()
                .find(|item| item.rating == rating)
                .map_or(0, |item| item.count),
        })
        .collect();
    Json(ProductFeedbackSummary {
        average_rating: aggregate.average_rating,
        total_reviews: aggregate.total_reviews,
        rating_counts,
        reviews,
    })
    .into_response()
}

#[utoipa::path(
    post,
    path = "/api/products/{slug}/feedback",
    tag = "product feedback",
    params(("slug" = String, Path)),
    request_body = CreateProductFeedbackRequest,
    responses(
        (status = 202, body = SubmittedProductFeedback),
        (status = 404, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(input): Json<CreateProductFeedbackRequest>,
) -> Response {
    let display_name = input.display_name.trim();
    let comment = input.comment.trim();
    if !(2..=100).contains(&display_name.chars().count())
        || !(10..=1200).contains(&comment.chars().count())
        || !(1..=5).contains(&input.rating)
    {
        return invalid();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let product_id = match active_product_id(&pool, &slug).await {
        Ok(Some(id)) => id,
        Ok(None) => return not_found(),
        Err(_) => return unavailable(),
    };
    let id = Uuid::now_v7();
    match sqlx::query(
        "INSERT INTO product_feedback (id,product_id,display_name,rating,comment) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(product_id)
    .bind(display_name)
    .bind(input.rating)
    .bind(comment)
    .execute(&pool)
    .await
    {
        Ok(_) => (
            StatusCode::ACCEPTED,
            Json(SubmittedProductFeedback {
                id,
                status: "pending".to_owned(),
            }),
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/feedback",
    tag = "admin product feedback",
    params(("status" = Option<String>, Query)),
    responses(
        (status = 200, body = [AdminProductFeedback]),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn admin_list(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Query(query): Query<AdminFeedbackQuery>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_READ) {
        return response.into_response();
    }
    let status = query.status.as_deref().unwrap_or("pending");
    if !matches!(status, "pending" | "approved" | "rejected" | "all") {
        return invalid_status();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    match admin_records(&pool, status).await {
        Ok(records) => Json(records).into_response(),
        Err(_) => unavailable(),
    }
}

#[utoipa::path(
    put,
    path = "/api/admin/feedback/{feedback_id}",
    tag = "admin product feedback",
    params(("feedback_id" = Uuid, Path)),
    request_body = ModerateProductFeedbackRequest,
    responses(
        (status = 200, body = AdminProductFeedback),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn moderate(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(feedback_id): Path<Uuid>,
    Json(input): Json<ModerateProductFeedbackRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    if !matches!(input.status.as_str(), "approved" | "rejected") {
        return invalid_status();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    let changed = sqlx::query(
        "UPDATE product_feedback SET status=$2,moderated_at=now(),moderated_by=$3 WHERE id=$1",
    )
    .bind(feedback_id)
    .bind(&input.status)
    .bind(actor.id)
    .execute(&mut *tx)
    .await;
    match changed {
        Ok(result) if result.rows_affected() == 0 => return not_found(),
        Ok(_) => {}
        Err(_) => return unavailable(),
    }
    if sqlx::query(
        "INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,metadata) VALUES ($1,$2,'product_feedback',$3,jsonb_build_object('status',$4::text))",
    )
    .bind(actor.id)
    .bind(format!("feedback.{}", input.status))
    .bind(feedback_id.to_string())
    .bind(&input.status)
    .execute(&mut *tx)
    .await
    .is_err()
        || tx.commit().await.is_err()
    {
        return unavailable();
    }
    match admin_record(&pool, feedback_id).await {
        Ok(Some(record)) => Json(record).into_response(),
        _ => unavailable(),
    }
}

#[utoipa::path(
    put,
    path = "/api/admin/feedback/{feedback_id}/reply",
    tag = "admin product feedback",
    params(("feedback_id" = Uuid, Path)),
    request_body = ReplyToProductFeedbackRequest,
    responses(
        (status = 200, body = AdminProductFeedback),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn reply(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(feedback_id): Path<Uuid>,
    Json(input): Json<ReplyToProductFeedbackRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, CATALOG_WRITE) {
        return response.into_response();
    }
    let normalized_reply = input
        .reply
        .as_deref()
        .map(str::trim)
        .filter(|reply| !reply.is_empty());
    if normalized_reply.is_some_and(|reply| !(2..=1200).contains(&reply.chars().count())) {
        return invalid_reply();
    }
    let Some(pool) = state.database else {
        return unavailable();
    };
    let status =
        match sqlx::query_scalar::<_, String>("SELECT status FROM product_feedback WHERE id=$1")
            .bind(feedback_id)
            .fetch_optional(&pool)
            .await
        {
            Ok(Some(status)) => status,
            Ok(None) => return not_found(),
            Err(_) => return unavailable(),
        };
    if status != "approved" {
        return error(
            StatusCode::CONFLICT,
            "feedback_not_approved",
            "Approve the feedback before publishing a store response.",
        );
    }
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable(),
    };
    if sqlx::query(
        r#"
        UPDATE product_feedback
        SET store_reply=$2,
            replied_at=CASE WHEN $2::text IS NULL THEN NULL ELSE now() END,
            replied_by=CASE WHEN $2::text IS NULL THEN NULL ELSE $3 END
        WHERE id=$1
        "#,
    )
    .bind(feedback_id)
    .bind(normalized_reply)
    .bind(actor.id)
    .execute(&mut *tx)
    .await
    .is_err()
    {
        return unavailable();
    }
    if sqlx::query(
        "INSERT INTO audit_log (actor_staff_user_id,action,entity_type,entity_id,metadata) VALUES ($1,$2,'product_feedback',$3,jsonb_build_object('has_reply',$4::bool))",
    )
    .bind(actor.id)
    .bind(if normalized_reply.is_some() { "feedback.reply" } else { "feedback.reply_remove" })
    .bind(feedback_id.to_string())
    .bind(normalized_reply.is_some())
    .execute(&mut *tx)
    .await
    .is_err()
        || tx.commit().await.is_err()
    {
        return unavailable();
    }
    match admin_record(&pool, feedback_id).await {
        Ok(Some(record)) => Json(record).into_response(),
        _ => unavailable(),
    }
}

async fn active_product_id(pool: &PgPool, slug: &str) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar("SELECT id FROM products WHERE slug=$1 AND status='active'")
        .bind(slug)
        .fetch_optional(pool)
        .await
}

fn admin_select() -> &'static str {
    r#"SELECT feedback.id,feedback.product_id,product.title AS product_title,
        product.slug AS product_slug,feedback.display_name,feedback.rating,feedback.comment,
        feedback.status,
        to_char(feedback.created_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
        CASE WHEN feedback.moderated_at IS NULL THEN NULL ELSE
            to_char(feedback.moderated_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') END AS moderated_at,
        moderator.display_name AS moderated_by_name,
        feedback.store_reply,
        CASE WHEN feedback.replied_at IS NULL THEN NULL ELSE
            to_char(feedback.replied_at AT TIME ZONE 'UTC','YYYY-MM-DD"T"HH24:MI:SS"Z"') END AS replied_at,
        replier.display_name AS replied_by_name
        FROM product_feedback feedback
        JOIN products product ON product.id=feedback.product_id
        LEFT JOIN staff_users moderator ON moderator.id=feedback.moderated_by
        LEFT JOIN staff_users replier ON replier.id=feedback.replied_by"#
}

async fn admin_records(
    pool: &PgPool,
    status: &str,
) -> Result<Vec<AdminProductFeedback>, sqlx::Error> {
    let sql = if status == "all" {
        format!(
            "{} ORDER BY feedback.created_at DESC LIMIT 300",
            admin_select()
        )
    } else {
        format!(
            "{} WHERE feedback.status=$1 ORDER BY feedback.created_at DESC LIMIT 300",
            admin_select()
        )
    };
    let query = sqlx::query_as::<_, AdminProductFeedback>(&sql);
    if status == "all" {
        query.fetch_all(pool).await
    } else {
        query.bind(status).fetch_all(pool).await
    }
}

async fn admin_record(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<AdminProductFeedback>, sqlx::Error> {
    sqlx::query_as(&format!("{} WHERE feedback.id=$1", admin_select()))
        .bind(id)
        .fetch_optional(pool)
        .await
}

fn invalid() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_feedback",
        "Provide a name, a rating from 1 to 5, and a comment between 10 and 1200 characters.",
    )
}

fn invalid_status() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_feedback_status",
        "Choose a valid feedback status.",
    )
}

fn invalid_reply() -> Response {
    error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invalid_feedback_reply",
        "Provide a response between 2 and 1200 characters, or remove it.",
    )
}

fn not_found() -> Response {
    error(
        StatusCode::NOT_FOUND,
        "feedback_not_found",
        "The product or feedback could not be found.",
    )
}

fn unavailable() -> Response {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "feedback_unavailable",
        "Product feedback is temporarily unavailable.",
    )
}

fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody::new(code, message))).into_response()
}

#[cfg(test)]
mod tests {
    #[test]
    fn feedback_limits_count_unicode_characters() {
        let name = "João";
        let comment = "Uma peça muito bonita.";
        assert!((2..=100).contains(&name.chars().count()));
        assert!((10..=1200).contains(&comment.chars().count()));
    }
}
