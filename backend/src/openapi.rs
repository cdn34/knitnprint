use utoipa::{OpenApi, openapi::OpenApi as OpenApiDocument};

use crate::{auth, catalog, error, health, media, staff};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "KnitPrint API",
        version = "0.1.0",
        description = "Authoritative API for the KnitPrint storefront and admin."
    ),
    paths(
        health::health,
        health::ready,
        auth::login,
        auth::logout,
        auth::me,
        catalog::admin_list,
        catalog::admin_detail,
        catalog::create,
        catalog::change_status,
        catalog::public_list,
        catalog::public_detail,
        media::initiate,
        media::complete,
        media::public_asset,
        staff::list,
        staff::create,
        staff::disable
    ),
    components(schemas(
        health::Health,
        error::ErrorBody,
        error::ErrorDetail,
        auth::LoginRequest,
        auth::StaffProfile,
        catalog::Variant,
        catalog::Product,
        catalog::ProductMedia,
        catalog::CreateProductRequest,
        catalog::CreateVariantRequest,
        catalog::ChangeProductStatusRequest,
        media::InitiateUploadRequest,
        media::InitiateUploadResponse,
        media::CompleteUploadRequest,
        media::MediaRecord,
        staff::StaffRecord,
        staff::CreateStaffRequest,
        staff::DisableStaffRequest
    )),
    tags(
        (name = "system", description = "Application health and readiness"),
        (name = "staff auth", description = "Private staff authentication"),
        (name = "staff", description = "Owner-authorized staff management"),
        (name = "admin catalog", description = "Capability-protected catalog management"),
        (name = "catalog", description = "Public published product catalog")
        ,(name = "admin media", description = "Capability-protected direct media uploads"),
        (name = "media", description = "Stable published product media")
    )
)]
struct ApiDoc;

pub fn document() -> OpenApiDocument {
    ApiDoc::openapi()
}
