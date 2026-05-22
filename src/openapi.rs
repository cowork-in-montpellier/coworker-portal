use utoipa::{
    OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify,
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Coworking Tooling API",
        version = "0.1.0",
        description = "Intranet portal for coworking space subscriptions and voucher management",
    ),
    components(schemas(
        crate::auth::routes::LoginRequest,
        crate::auth::routes::LoginResponse,
        crate::auth::routes::ForgotPasswordRequest,
        crate::auth::routes::ResetPasswordRequest,
        crate::domain::Service,
        crate::domain::VoucherSpec,
        crate::domain::VoucherStatus,
        crate::routes::services::ServicesResponse,
        crate::routes::bills::CreateBillRequest,
        crate::routes::bills::BillResponse,
        crate::routes::bills::VoucherResponse,
        crate::routes::bills::ListBillsResponse,
        crate::routes::vouchers::VoucherStatusResponse,
        crate::routes::vouchers::VoucherCheckResponse,
        crate::routes::profile::ProfileResponse,
        crate::routes::profile::UpdateProfileRequest,
        crate::routes::profile::ChangePasswordRequest,
        crate::routes::invitations::SendInvitationRequest,
        crate::auth::routes::AcceptInviteRequest,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication"),
        (name = "Services", description = "Available subscription services"),
        (name = "Bills", description = "Bill management"),
        (name = "Vouchers", description = "Voucher status and PDF generation"),
        (name = "Profile", description = "User profile management"),
        (name = "Invitations", description = "Member invitation management"),
    )
)]
pub struct ApiDoc;
