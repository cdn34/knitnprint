use utoipa::{OpenApi, openapi::OpenApi as OpenApiDocument};

use crate::{auth, catalog, error, health, staff};

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
        catalog::CreateProductRequest,
        catalog::CreateVariantRequest,
        catalog::ChangeProductStatusRequest,
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
    )
)]
struct ApiDoc;

pub fn document() -> OpenApiDocument {
    ApiDoc::openapi()
}
