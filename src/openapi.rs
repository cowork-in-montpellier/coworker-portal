use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Coworking Tooling API",
        version = "0.1.0",
        description = "Intranet portal for coworking space subscriptions and voucher management",
    )
)]
pub struct ApiDoc;
