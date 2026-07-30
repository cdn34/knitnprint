use utoipa::{OpenApi, openapi::OpenApi as OpenApiDocument};

use crate::{auth, error, health};

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
        auth::me
    ),
    components(schemas(
        health::Health,
        error::ErrorBody,
        error::ErrorDetail,
        auth::LoginRequest,
        auth::StaffProfile
    )),
    tags(
        (name = "system", description = "Application health and readiness"),
        (name = "staff auth", description = "Private staff authentication")
    )
)]
struct ApiDoc;

pub fn document() -> OpenApiDocument {
    ApiDoc::openapi()
}
