use utoipa::{OpenApi, openapi::OpenApi as OpenApiDocument};

use crate::{
    auth, carts, catalog, customer_auth, customers, error, health, inventory, media, orders, staff,
};

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
        customer_auth::register,
        customer_auth::login,
        customer_auth::logout,
        customer_auth::me,
        customer_auth::add_address,
        customer_auth::request_verification,
        customer_auth::confirm_verification,
        customer_auth::forgot_password,
        customer_auth::reset_password,
        carts::get,
        carts::add_item,
        carts::update_item,
        carts::remove_item,
        carts::set_delivery,
        orders::create,
        orders::admin_list,
        orders::admin_detail,
        orders::record_manual_payment,
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
        customer_auth::CustomerRegisterRequest,
        customer_auth::CustomerLoginRequest,
        customer_auth::CreateAccountAddressRequest,
        customer_auth::AccountTokenRequest,
        customer_auth::ForgotPasswordRequest,
        customer_auth::ResetPasswordRequest,
        customer_auth::CustomerAccountProfile,
        carts::AddCartItemRequest,
        carts::UpdateCartItemRequest,
        carts::Cart,
        carts::CartItem,
        carts::CartIssue,
        carts::CartDelivery,
        carts::CartAddress,
        orders::CreateOrderRequest,
        orders::ManualPaymentRequest,
        orders::Order,
        orders::OrderCustomer,
        orders::OrderAddress,
        orders::OrderLine,
        orders::OrderPayment,
        orders::OrderEvent,
        orders::OrderSummary,
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
        (name = "customer account", description = "Optional storefront customer authentication and owned data"),
        (name = "staff", description = "Owner-authorized staff management"),
        (name = "admin catalog", description = "Capability-protected catalog management"),
        (name = "catalog", description = "Public published product catalog")
        ,(name = "admin media", description = "Capability-protected direct media uploads"),
        (name = "media", description = "Stable published product media")
        ,(name = "inventory", description = "Variant availability and immutable stock movements")
        ,(name = "customers", description = "Guest customer contact capture")
        ,(name = "admin customers", description = "Privacy-aware customer inspection")
        ,(name = "cart", description = "Disposable server-priced checkout preparation")
        ,(name = "orders", description = "Idempotent cart conversion and order confirmation")
        ,(name = "admin orders", description = "Capability-protected order operations")
    )
)]
struct ApiDoc;

pub fn document() -> OpenApiDocument {
    ApiDoc::openapi()
}
