pub mod intention;
pub mod places;
pub mod rating;
pub mod today;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::lunch::state::State;

pub fn router() -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(today::get_today))
        .routes(routes!(places::create_place))
        .routes(routes!(places::update_place, places::delete_place))
        .routes(routes!(rating::set_rating))
        .routes(routes!(intention::set_intention))
}
