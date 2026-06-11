pub mod bills;
pub mod guest;
pub mod services;
pub mod status;
pub mod vouchers;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::invoice::state::State;

pub fn router() -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(status::status))
        .routes(routes!(services::list_services))
        .routes(routes!(bills::create_bill, bills::list_bills))
        .routes(routes!(bills::get_bill))
        .routes(routes!(vouchers::check_vouchers))
        .routes(routes!(vouchers::revoke_voucher))
        .routes(routes!(vouchers::bill_pdf))
        .routes(routes!(guest::list_guest_services))
        .routes(routes!(guest::create_guest_bill))
        .routes(routes!(guest::get_guest_bill))
        .routes(routes!(guest::check_guest_vouchers))
        .routes(routes!(guest::guest_bill_pdf))
}
