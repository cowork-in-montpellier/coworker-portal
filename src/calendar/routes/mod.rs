pub mod bookings;
pub mod rooms;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::calendar::state::State;

pub fn router() -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(rooms::list_rooms))
        .routes(routes!(rooms::room_calendar))
        .routes(routes!(rooms::all_calendar))
        .routes(routes!(bookings::list_bookings))
        .routes(routes!(bookings::create_booking))
        .routes(routes!(bookings::delete_booking))
}
