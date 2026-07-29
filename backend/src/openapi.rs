use utoipa::{OpenApi, openapi::OpenApi as OpenApiDocument};

use crate::{error, health};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "KnitPrint API",
        version = "0.1.0",
        description = "Authoritative API for the KnitPrint storefront and admin."
    ),
    paths(health::health, health::ready),
    components(schemas(health::Health, error::ErrorBody, error::ErrorDetail)),
    tags((name = "system", description = "Application health and readiness"))
)]
struct ApiDoc;

pub fn document() -> OpenApiDocument {
    ApiDoc::openapi()
}
