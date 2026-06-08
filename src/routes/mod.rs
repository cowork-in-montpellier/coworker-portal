use utoipa_axum::{router::OpenApiRouter, routes};
use crate::AppState;

pub mod bills;
pub mod bookings;
pub mod guest;
pub mod invitations;
pub mod profile;
pub mod rooms;
pub mod services;
pub mod status;
pub mod vouchers;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(status::status))
        .routes(routes!(profile::get_profile, profile::update_profile))
        .routes(routes!(profile::change_password))
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
        .routes(routes!(invitations::send_invitation))
        .routes(routes!(rooms::list_rooms))
        .routes(routes!(rooms::room_calendar))
        .routes(routes!(rooms::all_calendar))
        .routes(routes!(bookings::list_bookings))
        .routes(routes!(bookings::create_booking))
        .routes(routes!(bookings::delete_booking))
}
