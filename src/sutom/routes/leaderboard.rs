use std::cmp::Ordering;
use std::collections::HashMap;

use axum::{Json, extract::State};
use chrono::NaiveDate;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::sutom::{domain, repository, service, state::State as SutomState};
use crate::users::auth::CurrentUser;

#[derive(Serialize, ToSchema)]
pub struct LeaderboardEntry {
    pub user_id: i32,
    pub first_name: String,
    pub points: f64,
    pub games_played: i32,
}

#[derive(Serialize, ToSchema)]
pub struct LeaderboardResponse {
    pub since: NaiveDate,
    pub entries: Vec<LeaderboardEntry>,
}

#[utoipa::path(
    get,
    path = "/sutom/leaderboard",
    tag = "Sutom",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Points leaderboard over the last 30 days", body = LeaderboardResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_leaderboard(
    State(state): State<SutomState>,
    _user: CurrentUser,
) -> Result<Json<LeaderboardResponse>, AppError> {
    let today = service::today_paris();
    let since = service::history_window_start(today);
    let rows = repository::list_scored_attempts_since(&state.db, since).await?;

    let mut totals: HashMap<i32, (String, f64, i32)> = HashMap::new();
    for row in rows {
        let is_catchup = domain::is_catchup_play(row.game_date, row.updated_at);
        let points = domain::leaderboard_points(row.par, row.score, row.is_first, is_catchup);
        let entry = totals.entry(row.user_id).or_insert((row.first_name, 0.0, 0));
        entry.1 += points;
        entry.2 += 1;
    }

    let mut entries: Vec<LeaderboardEntry> = totals
        .into_iter()
        .map(|(user_id, (first_name, points, games_played))| LeaderboardEntry {
            user_id,
            first_name,
            points,
            games_played,
        })
        .collect();
    entries.sort_by(|a, b| b.points.partial_cmp(&a.points).unwrap_or(Ordering::Equal));

    Ok(Json(LeaderboardResponse { since, entries }))
}
