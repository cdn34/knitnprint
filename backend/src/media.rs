use std::{env, time::Duration};

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client, config::Region, presigning::PresigningConfig};
use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedStaff, require_capability},
    error::ErrorBody,
};

const MAX_PRODUCT_IMAGE_BYTES: i64 = 10 * 1024 * 1024;
const MEDIA_UPLOAD: &str = "media.upload";

#[derive(Clone)]
pub struct MediaStorage {
    pub client: Client,
    pub bucket: String,
}

impl MediaStorage {
    pub async fn from_env(production: bool) -> Result<Option<Self>, String> {
        let values = (
            env::var("S3_ENDPOINT").ok(),
            env::var("S3_REGION").ok(),
            env::var("S3_BUCKET").ok(),
            env::var("S3_ACCESS_KEY_ID").ok(),
            env::var("S3_SECRET_ACCESS_KEY").ok(),
        );
        let (endpoint, region, bucket, access_key, secret_key) = match values {
            (Some(endpoint), Some(region), Some(bucket), Some(access_key), Some(secret_key)) => {
                (endpoint, region, bucket, access_key, secret_key)
            }
            _ if production => {
                return Err("S3 storage configuration is required in production".into());
            }
            _ => (
                "http://127.0.0.1:9100".into(),
                "eu-west-1".into(),
                "knitprint-media".into(),
                "knitprint".into(),
                "knitprint-local".into(),
            ),
        };
        let shared = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "knitprint-config",
            ))
            .load()
            .await;
        let config = aws_sdk_s3::config::Builder::from(&shared)
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();
        Ok(Some(Self {
            client: Client::from_conf(config),
            bucket,
        }))
    }
}

#[derive(Deserialize, ToSchema)]
pub struct InitiateUploadRequest {
    pub filename: String,
    pub content_type: String,
    pub byte_size: i64,
}

#[derive(Serialize, ToSchema)]
pub struct InitiateUploadResponse {
    pub id: Uuid,
    pub upload_url: String,
    pub method: String,
    pub expires_in_seconds: u64,
}

#[derive(Deserialize, ToSchema)]
pub struct CompleteUploadRequest {
    pub product_id: Uuid,
    pub alt_text: String,
}

#[derive(Serialize, ToSchema)]
pub struct MediaRecord {
    pub id: Uuid,
    pub product_id: Uuid,
    pub alt_text: String,
    pub position: i32,
}

#[utoipa::path(
    post,
    path = "/api/admin/media/uploads",
    tag = "admin media",
    request_body = InitiateUploadRequest,
    responses(
        (status = 201, body = InitiateUploadResponse),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 422, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn initiate(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Json(input): Json<InitiateUploadRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, MEDIA_UPLOAD) {
        return response.into_response();
    }
    if !valid_upload(&input) {
        return invalid_upload();
    }
    let (Some(pool), Some(storage)) = (state.database, state.media_storage) else {
        return unavailable();
    };
    let id = Uuid::now_v7();
    let extension = extension_for(&input.content_type);
    let object_key = format!("uploads-quarantine/{id}/original.{extension}");
    let presigned = storage
        .client
        .put_object()
        .bucket(&storage.bucket)
        .key(&object_key)
        .content_type(&input.content_type)
        .presigned(
            PresigningConfig::expires_in(Duration::from_secs(300))
                .expect("five minutes is a valid presigning duration"),
        )
        .await;
    let Ok(presigned) = presigned else {
        return unavailable();
    };
    if sqlx::query(
        r#"
        INSERT INTO media_assets (
            id, object_key, content_type, byte_size, created_by_staff_user_id
        )
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id)
    .bind(object_key)
    .bind(&input.content_type)
    .bind(input.byte_size)
    .bind(actor.id)
    .execute(&pool)
    .await
    .is_err()
    {
        return unavailable();
    }
    (
        StatusCode::CREATED,
        Json(InitiateUploadResponse {
            id,
            upload_url: presigned.uri().to_string(),
            method: "PUT".into(),
            expires_in_seconds: 300,
        }),
    )
        .into_response()
}

#[utoipa::path(
    post,
    path = "/api/admin/media/uploads/{media_id}/complete",
    params(("media_id" = Uuid, Path)),
    tag = "admin media",
    request_body = CompleteUploadRequest,
    responses(
        (status = 200, body = MediaRecord),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 422, body = ErrorBody)
    )
)]
pub async fn complete(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path(media_id): Path<Uuid>,
    Json(input): Json<CompleteUploadRequest>,
) -> Response {
    if let Err(response) = require_capability(&actor, MEDIA_UPLOAD) {
        return response.into_response();
    }
    let alt_text = input.alt_text.trim();
    if alt_text.is_empty() || alt_text.len() > 500 {
        return invalid_upload();
    }
    let (Some(pool), Some(storage)) = (state.database, state.media_storage) else {
        return unavailable();
    };
    let asset = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT object_key, content_type, byte_size FROM media_assets WHERE id = $1 AND status = 'pending'",
    )
    .bind(media_id)
    .fetch_optional(&pool)
    .await;
    let (object_key, content_type, byte_size) = match asset {
        Ok(Some(asset)) => asset,
        Ok(None) => return not_found(),
        Err(_) => return unavailable(),
    };
    let head = storage
        .client
        .head_object()
        .bucket(&storage.bucket)
        .key(&object_key)
        .send()
        .await;
    let Ok(head) = head else {
        return upload_incomplete();
    };
    if head.content_length() != Some(byte_size)
        || head
            .content_type()
            .is_some_and(|actual| actual != content_type)
    {
        return upload_mismatch();
    }
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let position: i32 = match sqlx::query_scalar(
        "SELECT COALESCE(max(position) + 1, 0) FROM product_media WHERE product_id = $1",
    )
    .bind(input.product_id)
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(position) => position,
        Err(_) => return unavailable(),
    };
    if sqlx::query("UPDATE media_assets SET status = 'ready', completed_at = now() WHERE id = $1")
        .bind(media_id)
        .execute(&mut *transaction)
        .await
        .is_err()
        || sqlx::query(
            "INSERT INTO product_media (product_id, media_asset_id, alt_text, position) VALUES ($1, $2, $3, $4)",
        )
        .bind(input.product_id)
        .bind(media_id)
        .bind(alt_text)
        .bind(position)
        .execute(&mut *transaction)
        .await
        .is_err()
        || sqlx::query(
            r#"
            INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id)
            VALUES ($1, 'media.complete', 'media_asset', $2)
            "#,
        )
        .bind(actor.id)
        .bind(media_id.to_string())
        .execute(&mut *transaction)
        .await
        .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    Json(MediaRecord {
        id: media_id,
        product_id: input.product_id,
        alt_text: alt_text.into(),
        position,
    })
    .into_response()
}

#[utoipa::path(
    get,
    path = "/api/media/{media_id}",
    params(("media_id" = Uuid, Path)),
    tag = "media",
    responses(
        (status = 200, description = "Immutable published product image"),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn public_asset(State(state): State<AppState>, Path(media_id): Path<Uuid>) -> Response {
    let (Some(pool), Some(storage)) = (state.database, state.media_storage) else {
        return unavailable();
    };
    let asset = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT m.object_key, m.content_type
        FROM media_assets m
        JOIN product_media pm ON pm.media_asset_id = m.id
        JOIN products p ON p.id = pm.product_id
        WHERE m.id = $1 AND m.status = 'ready' AND p.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(media_id)
    .fetch_optional(&pool)
    .await;
    let (object_key, content_type) = match asset {
        Ok(Some(asset)) => asset,
        Ok(None) => return not_found(),
        Err(_) => return unavailable(),
    };
    let object = storage
        .client
        .get_object()
        .bucket(&storage.bucket)
        .key(object_key)
        .send()
        .await;
    let Ok(object) = object else {
        return not_found();
    };
    let etag = object.e_tag().map(ToOwned::to_owned);
    let body = match object.body.collect().await {
        Ok(body) => body.into_bytes(),
        Err(_) => return unavailable(),
    };
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        content_type
            .parse()
            .unwrap_or_else(|_| header::HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    if let Some(etag) = etag
        && let Ok(etag) = etag.parse()
    {
        response.headers_mut().insert(header::ETAG, etag);
    }
    response
}

fn valid_upload(input: &InitiateUploadRequest) -> bool {
    !input.filename.trim().is_empty()
        && input.filename.len() <= 255
        && matches!(
            input.content_type.as_str(),
            "image/jpeg" | "image/png" | "image/webp"
        )
        && (1..=MAX_PRODUCT_IMAGE_BYTES).contains(&input.byte_size)
}

fn extension_for(content_type: &str) -> &'static str {
    match content_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "bin",
    }
}

fn invalid_upload() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorBody::new(
            "invalid_media_upload",
            "Use a JPEG, PNG, or WebP image up to 10 MB and provide alt text.",
        )),
    )
        .into_response()
}

fn upload_incomplete() -> Response {
    (
        StatusCode::CONFLICT,
        Json(ErrorBody::new(
            "upload_incomplete",
            "The uploaded object could not be verified.",
        )),
    )
        .into_response()
}

fn upload_mismatch() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorBody::new(
            "upload_mismatch",
            "The uploaded object does not match the declared file.",
        )),
    )
        .into_response()
}

fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody::new(
            "media_not_found",
            "The pending media upload was not found.",
        )),
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody::new(
            "media_unavailable",
            "Media storage is temporarily unavailable.",
        )),
    )
        .into_response()
}
