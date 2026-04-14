use utoipa_axum::{router::OpenApiRouter, routes};
use crate::AppState;

pub mod bills;
pub mod services;
pub mod vouchers;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(services::list_services))
        .routes(routes!(bills::create_bill, bills::list_bills))
        .routes(routes!(bills::get_bill))
        .routes(routes!(vouchers::check_vouchers))
        .routes(routes!(vouchers::bill_pdf))
}
