pub mod attempt;
pub mod day;
pub mod history;
pub mod leaderboard;
pub mod scoreboard;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::sutom::state::State;

pub fn router() -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(day::get_today))
        .routes(routes!(day::get_day))
        .routes(routes!(attempt::submit_guesses))
        .routes(routes!(scoreboard::get_scoreboard))
        .routes(routes!(leaderboard::get_leaderboard))
        .routes(routes!(history::get_history))
}
