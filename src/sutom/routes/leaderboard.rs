use std::collections::HashMap;

use axum::{Json, extract::{Query, State}};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::sutom::{domain, repository, service, state::State as SutomState};
use crate::users::auth::CurrentUser;

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LeaderboardQuery {
    /// Month to rank, `YYYY-MM`. Defaults to the current month (Paris time).
    pub month: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct LeaderboardEntry {
    pub user_id: i32,
    pub first_name: String,
    /// First letter of the player's last name (e.g. disambiguates two "Alice"s).
    pub last_initial: String,
    pub points: f64,
    pub games_played: i32,
    /// Average number of guesses per word actually played in the month (a failed
    /// attempt counts as `MAX_ATTEMPTS + 1`). Days not played are skipped rather than
    /// counted against the player. Breaks ties between equal point totals.
    pub avg_guesses: f64,
    /// Months the player has won to date, whichever month is being shown.
    pub stars: i32,
}

#[derive(Serialize, ToSchema)]
pub struct LeaderboardResponse {
    /// The ranked month, `YYYY-MM`.
    pub month: String,
    /// True for a month that is over: its ranking is final and later catch-ups on its
    /// grids no longer count.
    pub is_final: bool,
    pub entries: Vec<LeaderboardEntry>,
}

#[utoipa::path(
    get,
    path = "/sutom/leaderboard",
    tag = "Sutom",
    params(LeaderboardQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Points leaderboard for one calendar month", body = LeaderboardResponse),
        (status = 400, description = "Malformed month"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_leaderboard(
    State(state): State<SutomState>,
    Query(q): Query<LeaderboardQuery>,
    _user: CurrentUser,
) -> Result<Json<LeaderboardResponse>, AppError> {
    let today = service::today_paris();
    let month_day = match q.month.as_deref() {
        Some(m) => NaiveDate::parse_from_str(&format!("{m}-01"), "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Mois invalide, format attendu : AAAA-MM.".into()))?,
        None => today,
    };
    let (month, _) = domain::month_bounds(month_day);
    let is_final = month < domain::month_bounds(today).0;

    // A month that is over is served from its frozen ranking. Until the nightly task has
    // frozen it, it is computed the same way the task would.
    let mut ranking = if is_final { repository::month_ranking(&state.db, month).await? } else { Vec::new() };
    if ranking.is_empty() {
        ranking = service::compute_month_ranking(&state, month, today).await?;
    }

    let stars: HashMap<i32, i64> = repository::star_counts(&state.db).await?.into_iter().collect();
    let entries = ranking
        .into_iter()
        .map(|e| LeaderboardEntry {
            last_initial: domain::last_initial(&e.last_name),
            user_id: e.user_id,
            first_name: e.first_name,
            points: e.points,
            games_played: e.games_played,
            avg_guesses: e.avg_guesses,
            stars: stars.get(&e.user_id).copied().unwrap_or(0) as i32,
        })
        .collect();

    Ok(Json(LeaderboardResponse { month: month.format("%Y-%m").to_string(), is_final, entries }))
}
