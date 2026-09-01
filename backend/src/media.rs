use std::{env, io::Cursor, time::Duration};

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client, config::Region, presigning::PresigningConfig, primitives::ByteStream};
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
    media_scanner::ScanOutcome,
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
        let endpoint = env::var("S3_ENDPOINT").ok();
        let region = env::var("S3_REGION")
            .ok()
            .or_else(|| (!production).then(|| "eu-west-1".into()));
        let bucket = env::var("S3_BUCKET")
            .ok()
            .or_else(|| (!production).then(|| "knitprint-media".into()));
        let access_key = env::var("S3_ACCESS_KEY_ID").ok();
        let secret_key = env::var("S3_SECRET_ACCESS_KEY").ok();
        let (Some(region), Some(bucket)) = (region, bucket) else {
            return Err("S3_REGION and S3_BUCKET are required in production".into());
        };
        if access_key.is_some() != secret_key.is_some() {
            return Err(
                "S3_ACCESS_KEY_ID and S3_SECRET_ACCESS_KEY must be configured together".into(),
            );
        }
        let mut loader =
            aws_config::defaults(BehaviorVersion::latest()).region(Region::new(region));
        if let (Some(access_key), Some(secret_key)) = (access_key, secret_key) {
            loader = loader.credentials_provider(Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "knitprint-config",
            ));
        } else if !production {
            loader = loader.credentials_provider(Credentials::new(
                "knitprint",
                "knitprint-local",
                None,
                None,
                "knitprint-development",
            ));
        }
        let shared = loader.load().await;
        let mut builder = aws_sdk_s3::config::Builder::from(&shared);
        if let Some(endpoint) =
            endpoint.or_else(|| (!production).then(|| "http://127.0.0.1:9100".into()))
        {
            builder = builder.endpoint_url(endpoint).force_path_style(true);
        }
        let config = builder.build();
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

#[derive(Serialize, ToSchema)]
pub struct PersonalizationMediaRecord {
    pub id: Uuid,
    pub preview_url: String,
}

struct ProcessedVariant {
    kind: &'static str,
    bytes: Vec<u8>,
    width: u32,
    height: u32,
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
        .content_length(input.byte_size)
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

#[utoipa::path(post, path = "/api/personalization/uploads", tag = "personalization", request_body = InitiateUploadRequest, responses((status = 201, body = InitiateUploadResponse), (status = 422, body = ErrorBody), (status = 503, body = ErrorBody)))]
pub async fn initiate_personalization(
    State(state): State<AppState>,
    Json(input): Json<InitiateUploadRequest>,
) -> Response {
    if !valid_upload(&input) {
        return invalid_upload();
    }
    let (Some(pool), Some(storage)) = (state.database, state.media_storage) else {
        return unavailable();
    };
    let id = Uuid::now_v7();
    let object_key = format!(
        "personalization-quarantine/{id}/original.{}",
        extension_for(&input.content_type)
    );
    let presigned = storage
        .client
        .put_object()
        .bucket(&storage.bucket)
        .key(&object_key)
        .content_type(&input.content_type)
        .content_length(input.byte_size)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(300)).expect("valid duration"))
        .await;
    let Ok(presigned) = presigned else {
        return unavailable();
    };
    if sqlx::query(
        "INSERT INTO media_assets (id, object_key, content_type, byte_size) VALUES ($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(object_key)
    .bind(&input.content_type)
    .bind(input.byte_size)
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
    let alt_text = input.alt_text.trim().to_owned();
    if alt_text.is_empty() || alt_text.len() > 500 {
        return invalid_upload();
    }
    complete_upload(
        state,
        media_id,
        Some((input.product_id, alt_text)),
        Some(actor.id),
    )
    .await
}

#[utoipa::path(post, path = "/api/personalization/uploads/{media_id}/complete", params(("media_id" = Uuid, Path)), tag = "personalization", responses((status = 200, body = PersonalizationMediaRecord), (status = 404, body = ErrorBody), (status = 422, body = ErrorBody), (status = 503, body = ErrorBody)))]
pub async fn complete_personalization(
    State(state): State<AppState>,
    Path(media_id): Path<Uuid>,
) -> Response {
    complete_upload(state, media_id, None, None).await
}

async fn complete_upload(
    state: AppState,
    media_id: Uuid,
    attachment: Option<(Uuid, String)>,
    actor_id: Option<Uuid>,
) -> Response {
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
    let source = storage
        .client
        .get_object()
        .bucket(&storage.bucket)
        .key(&object_key)
        .send()
        .await;
    let source = match source {
        Ok(source) => match source.body.collect().await {
            Ok(body) => body.into_bytes().to_vec(),
            Err(_) => return upload_incomplete(),
        },
        Err(_) => return upload_incomplete(),
    };
    match state.media_scanner.scan(&source).await {
        Ok(ScanOutcome::Clean) => {}
        Ok(ScanOutcome::Infected(signature)) => {
            return reject_infected(&pool, media_id, actor_id, &signature).await;
        }
        Err(error) => {
            tracing::warn!(media_id = %media_id, %error, "media malware scan failed closed");
            return scanner_unavailable();
        }
    }
    if !declared_format_matches(&source, &content_type) {
        return invalid_image();
    }
    let variants = match tokio::task::spawn_blocking(move || process_image(&source)).await {
        Ok(Ok(variants)) => variants,
        _ => return invalid_image(),
    };
    for variant in &variants {
        if storage
            .client
            .put_object()
            .bucket(&storage.bucket)
            .key(variant_key(media_id, variant.kind))
            .content_type("image/webp")
            .body(ByteStream::from(variant.bytes.clone()))
            .send()
            .await
            .is_err()
        {
            return unavailable();
        }
    }
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let position: i32 = if let Some((product_id, _)) = &attachment {
        match sqlx::query_scalar(
            "SELECT COALESCE(max(position) + 1, 0) FROM product_media WHERE product_id = $1",
        )
        .bind(product_id)
        .fetch_one(&mut *transaction)
        .await
        {
            Ok(position) => position,
            Err(_) => return unavailable(),
        }
    } else {
        0
    };
    for variant in &variants {
        if sqlx::query(
            r#"
            INSERT INTO media_variants (
                media_asset_id, kind, object_key, byte_size, width, height
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(media_id)
        .bind(variant.kind)
        .bind(variant_key(media_id, variant.kind))
        .bind(variant.bytes.len() as i64)
        .bind(variant.width as i32)
        .bind(variant.height as i32)
        .execute(&mut *transaction)
        .await
        .is_err()
        {
            return unavailable();
        }
    }
    if sqlx::query("UPDATE media_assets SET status = 'ready', scan_status = 'clean', scanned_at = now(), scan_detail = NULL, completed_at = now() WHERE id = $1")
        .bind(media_id)
        .execute(&mut *transaction)
        .await
        .is_err()
    {
        return unavailable();
    }
    if let Some((product_id, alt_text)) = &attachment
        && sqlx::query("INSERT INTO product_media (product_id, media_asset_id, alt_text, position) VALUES ($1, $2, $3, $4)")
            .bind(product_id).bind(media_id).bind(alt_text).bind(position)
            .execute(&mut *transaction).await.is_err()
    { return unavailable(); }
    if sqlx::query(
        r#"
            INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id)
            VALUES ($1, 'media.complete', 'media_asset', $2)
            "#,
    )
    .bind(actor_id)
    .bind(media_id.to_string())
    .execute(&mut *transaction)
    .await
    .is_err()
        || transaction.commit().await.is_err()
    {
        return unavailable();
    }
    if let Some((product_id, alt_text)) = attachment {
        Json(MediaRecord {
            id: media_id,
            product_id,
            alt_text,
            position,
        })
        .into_response()
    } else {
        Json(PersonalizationMediaRecord {
            id: media_id,
            preview_url: format!("/api/admin/personalization/media/{media_id}/detail"),
        })
        .into_response()
    }
}

#[utoipa::path(
    get,
    path = "/api/media/{media_id}/{variant}",
    params(("media_id" = Uuid, Path), ("variant" = String, Path)),
    tag = "media",
    responses(
        (status = 200, description = "Immutable published product image"),
        (status = 404, body = ErrorBody),
        (status = 503, body = ErrorBody)
    )
)]
pub async fn public_asset(
    State(state): State<AppState>,
    Path((media_id, variant)): Path<(Uuid, String)>,
) -> Response {
    if !matches!(variant.as_str(), "thumbnail" | "card" | "detail") {
        return not_found();
    }
    let (Some(pool), Some(storage)) = (state.database, state.media_storage) else {
        return unavailable();
    };
    let asset = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT mv.object_key, mv.content_type
        FROM media_assets m
        JOIN media_variants mv ON mv.media_asset_id = m.id
        JOIN product_media pm ON pm.media_asset_id = m.id
        JOIN products p ON p.id = pm.product_id
        WHERE m.id = $1 AND mv.kind = $2 AND m.status = 'ready' AND p.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(media_id)
    .bind(&variant)
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

#[utoipa::path(get, path = "/api/admin/personalization/media/{media_id}/{variant}", params(("media_id" = Uuid, Path), ("variant" = String, Path)), tag = "admin personalization", responses((status = 200, description = "Processed customer personalization image"), (status = 401, body = ErrorBody), (status = 403, body = ErrorBody), (status = 404, body = ErrorBody)))]
pub async fn admin_personalization_asset(
    State(state): State<AppState>,
    actor: AuthenticatedStaff,
    Path((media_id, variant)): Path<(Uuid, String)>,
) -> Response {
    if let Err(response) = require_capability(&actor, "orders.read") {
        return response.into_response();
    }
    if !matches!(variant.as_str(), "thumbnail" | "detail") {
        return not_found();
    }
    let (Some(pool), Some(storage)) = (state.database, state.media_storage) else {
        return unavailable();
    };
    let asset = sqlx::query_as::<_, (String, String)>(
        "SELECT mv.object_key, mv.content_type FROM media_assets m JOIN media_variants mv ON mv.media_asset_id=m.id WHERE m.id=$1 AND mv.kind=$2 AND m.status='ready' AND NOT EXISTS (SELECT 1 FROM product_media pm WHERE pm.media_asset_id=m.id) LIMIT 1"
    ).bind(media_id).bind(&variant).fetch_optional(&pool).await;
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
    let body = match object.body.collect().await {
        Ok(body) => body.into_bytes(),
        Err(_) => return unavailable(),
    };
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        content_type
            .parse()
            .unwrap_or_else(|_| header::HeaderValue::from_static("image/webp")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, no-store"),
    );
    response
}

fn process_image(source: &[u8]) -> Result<Vec<ProcessedVariant>, image::ImageError> {
    let format = image::guess_format(source)?;
    let dimensions =
        image::ImageReader::with_format(Cursor::new(source), format).into_dimensions()?;
    let pixels = u64::from(dimensions.0) * u64::from(dimensions.1);
    if pixels > 40_000_000 {
        return Err(image::ImageError::Limits(
            image::error::LimitError::from_kind(image::error::LimitErrorKind::DimensionError),
        ));
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(source), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    limits.max_alloc = Some(200 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?;
    [("thumbnail", 320), ("card", 900), ("detail", 1600)]
        .into_iter()
        .map(|(kind, maximum)| {
            let resized = image.thumbnail(maximum, maximum);
            let mut output = Cursor::new(Vec::new());
            resized.write_to(&mut output, image::ImageFormat::WebP)?;
            Ok(ProcessedVariant {
                kind,
                bytes: output.into_inner(),
                width: resized.width(),
                height: resized.height(),
            })
        })
        .collect()
}

fn declared_format_matches(source: &[u8], content_type: &str) -> bool {
    let expected = match content_type {
        "image/jpeg" => image::ImageFormat::Jpeg,
        "image/png" => image::ImageFormat::Png,
        "image/webp" => image::ImageFormat::WebP,
        _ => return false,
    };
    image::guess_format(source).is_ok_and(|actual| actual == expected)
}

async fn reject_infected(
    pool: &sqlx::PgPool,
    media_id: Uuid,
    actor_id: Option<Uuid>,
    signature: &str,
) -> Response {
    let detail = signature.chars().take(200).collect::<String>();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return unavailable(),
    };
    let updated = sqlx::query(
        "UPDATE media_assets SET status = 'failed', scan_status = 'infected', scanned_at = now(), scan_detail = $2 WHERE id = $1 AND status = 'pending'",
    )
    .bind(media_id)
    .bind(&detail)
    .execute(&mut *transaction)
    .await;
    let audited = sqlx::query(
        "INSERT INTO audit_log (actor_staff_user_id, action, entity_type, entity_id, reason) VALUES ($1, 'media.scan_rejected', 'media_asset', $2, 'Malware scanner rejected quarantined upload')",
    )
    .bind(actor_id)
    .bind(media_id.to_string())
    .execute(&mut *transaction)
    .await;
    if updated.is_err() || audited.is_err() || transaction.commit().await.is_err() {
        return unavailable();
    }
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorBody::new(
            "unsafe_media_upload",
            "The quarantined upload failed the safety scan and was rejected.",
        )),
    )
        .into_response()
}

fn variant_key(media_id: Uuid, kind: &str) -> String {
    format!("media-public/{media_id}/{kind}.webp")
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

fn invalid_image() -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ErrorBody::new(
            "invalid_image",
            "The uploaded file is not a supported image or exceeds the pixel limit.",
        )),
    )
        .into_response()
}

fn scanner_unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody::new(
            "media_scanner_unavailable",
            "The upload remains quarantined because the safety scan could not complete.",
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat, RgbaImage};

    use super::{declared_format_matches, process_image};

    fn encoded(format: ImageFormat) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(RgbaImage::new(2, 2));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, format).unwrap();
        bytes.into_inner()
    }

    #[test]
    fn declared_content_type_must_match_image_signature() {
        let png = encoded(ImageFormat::Png);
        assert!(declared_format_matches(&png, "image/png"));
        assert!(!declared_format_matches(&png, "image/jpeg"));
    }

    #[test]
    fn decoded_images_are_normalized_to_bounded_webp_variants() {
        let png = encoded(ImageFormat::Png);
        let variants = process_image(&png).unwrap();
        assert_eq!(variants.len(), 3);
        for (variant, maximum) in variants.iter().zip([320, 900, 1600]) {
            assert!((1..=maximum).contains(&variant.width));
            assert!((1..=maximum).contains(&variant.height));
        }
    }
}
