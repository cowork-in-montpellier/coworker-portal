pub mod today;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::sutom::state::State;

pub fn router() -> OpenApiRouter<State> {
    OpenApiRouter::new().routes(routes!(today::get_today))
}
