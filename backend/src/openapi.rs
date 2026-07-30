use utoipa::{OpenApi, openapi::OpenApi as OpenApiDocument};

use crate::{auth, error, health, staff};

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
        staff::StaffRecord,
        staff::CreateStaffRequest,
        staff::DisableStaffRequest
    )),
    tags(
        (name = "system", description = "Application health and readiness"),
        (name = "staff auth", description = "Private staff authentication"),
        (name = "staff", description = "Owner-authorized staff management")
    )
)]
struct ApiDoc;

pub fn document() -> OpenApiDocument {
    ApiDoc::openapi()
}
