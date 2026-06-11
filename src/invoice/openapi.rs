use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
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
    components(schemas(
        crate::invoice::domain::Service,
        crate::invoice::domain::VoucherSpec,
        crate::invoice::domain::VoucherStatus,
        crate::invoice::routes::status::StatusResponse,
        crate::invoice::routes::services::ServicesResponse,
        crate::invoice::routes::bills::CreateBillRequest,
        crate::invoice::routes::bills::BillResponse,
        crate::invoice::routes::bills::VoucherResponse,
        crate::invoice::routes::bills::ListBillsResponse,
        crate::invoice::routes::vouchers::VoucherStatusResponse,
        crate::invoice::routes::vouchers::VoucherCheckResponse,
        crate::invoice::routes::vouchers::RevokeVoucherResponse,
        crate::invoice::routes::guest::GuestServicesResponse,
        crate::invoice::routes::guest::GuestVoucherResponse,
        crate::invoice::routes::guest::GuestBillLineResponse,
        crate::invoice::routes::guest::GuestBillResponse,
        crate::invoice::routes::guest::CreateGuestBillRequest,
        crate::invoice::routes::guest::CreateGuestBillLineRequest,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Status", description = "Application feature availability"),
        (name = "Services", description = "Available subscription services"),
        (name = "Bills", description = "Bill management"),
        (name = "Vouchers", description = "Voucher status and PDF generation"),
        (name = "Guest", description = "Guest bill and voucher management"),
    )
)]
pub struct ApiDoc;
