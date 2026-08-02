use utoipa::{OpenApi, openapi::OpenApi as OpenApiDocument};

use crate::{auth, catalog, customers, error, health, inventory, media, staff};

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
        catalog::category_list,
        catalog::category_create,
        catalog::add_variant,
        catalog::assign_categories,
        catalog::public_list,
        catalog::public_category_list,
        catalog::public_detail,
        media::initiate,
        media::complete,
        media::public_asset,
        inventory::list,
        inventory::movements,
        inventory::adjust,
        customers::create_guest,
        customers::list,
        customers::detail,
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
        catalog::Category,
        catalog::Product,
        catalog::ProductMedia,
        catalog::CreateProductRequest,
        catalog::CreateVariantRequest,
        catalog::ChangeProductStatusRequest,
        catalog::CreateCategoryRequest,
        catalog::AssignCategoriesRequest,
        media::InitiateUploadRequest,
        media::InitiateUploadResponse,
        media::CompleteUploadRequest,
        media::MediaRecord,
        inventory::InventoryRecord,
        inventory::InventoryMovement,
        inventory::AdjustInventoryRequest,
        customers::GuestCustomerRequest,
        customers::CustomerAddressInput,
        customers::GuestCustomerReceipt,
        customers::CustomerSummary,
        customers::CustomerAddress,
        customers::CustomerDetail,
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
        ,(name = "inventory", description = "Variant availability and immutable stock movements")
        ,(name = "customers", description = "Guest customer contact capture")
        ,(name = "admin customers", description = "Privacy-aware customer inspection")
    )
)]
struct ApiDoc;

pub fn document() -> OpenApiDocument {
    ApiDoc::openapi()
}
