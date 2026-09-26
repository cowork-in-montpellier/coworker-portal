use std::cmp::Ordering;
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
    /// Include today's still-in-progress results, scored against today's live
    /// (not yet frozen) par estimate. Defaults to true.
    #[serde(default = "default_include_today")]
    pub include_today: bool,
}
fn default_include_today() -> bool {
    true
}

#[derive(Serialize, ToSchema)]
pub struct LeaderboardEntry {
    pub user_id: i32,
    pub first_name: String,
    /// First letter of the player's last name (e.g. disambiguates two "Alice"s).
    pub last_initial: String,
    pub points: f64,
    pub games_played: i32,
    /// Average number of guesses per word actually played in the window (a failed
    /// attempt counts as `MAX_ATTEMPTS + 1`). Days not played are skipped entirely
    /// rather than counted against the player, so this isn't ordered the same way
    /// points are — purely informational, not used for ranking.
    pub avg_guesses: f64,
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
    params(LeaderboardQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Points leaderboard since the competition's start", body = LeaderboardResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_leaderboard(
    State(state): State<SutomState>,
    Query(q): Query<LeaderboardQuery>,
    _user: CurrentUser,
) -> Result<Json<LeaderboardResponse>, AppError> {
    let today = service::today_paris();
    // Never look further back than the competition's official start — earlier days
    // were warm-up/training and never count toward leaderboard points.
    let since = service::history_window_start(today).max(domain::leaderboard_start_date());
    let mut rows = repository::list_scored_attempts_since(&state.db, since).await?;

    // Today's par is never frozen yet (that only happens overnight), so it's excluded
    // by `list_scored_attempts_since`'s `w.par IS NOT NULL` filter above. Optionally
    // fold it back in here, scored against today's live estimate instead.
    if q.include_today && today >= since {
        let today_attempts = repository::list_attempts_for_date(&state.db, today).await?;
        let finished_today: Vec<_> = today_attempts.into_iter().filter(|a| a.score.is_some()).collect();
        if let Some(par) = repository::average_score_ceil(&state.db, today).await? {
            let first_finisher_id = finished_today.iter().min_by_key(|a| a.updated_at).map(|a| a.user_id);
            for a in finished_today {
                rows.push(repository::ScoreRow {
                    user_id: a.user_id,
                    first_name: a.first_name,
                    last_name: a.last_name,
                    score: a.score.unwrap(),
                    par,
                    game_date: today,
                    updated_at: a.updated_at,
                    is_first: Some(a.user_id) == first_finisher_id,
                });
            }
        }
    }

    let mut totals: HashMap<i32, (String, String, f64, i32, i32)> = HashMap::new();
    for row in rows {
        let is_catchup = domain::is_catchup_play(row.game_date, row.updated_at);
        let points = domain::leaderboard_points(row.par, row.score, row.is_first, is_catchup);
        let entry = totals.entry(row.user_id).or_insert((row.first_name, row.last_name, 0.0, 0, 0));
        entry.2 += points;
        entry.3 += 1;
        entry.4 += row.score;
    }

    let mut entries: Vec<LeaderboardEntry> = totals
        .into_iter()
        .map(|(user_id, (first_name, last_name, points, games_played, score_sum))| LeaderboardEntry {
            user_id,
            first_name,
            last_initial: domain::last_initial(&last_name),
            points,
            games_played,
            avg_guesses: score_sum as f64 / games_played as f64,
        })
        .collect();
    entries.sort_by(|a, b| b.points.partial_cmp(&a.points).unwrap_or(Ordering::Equal));

    Ok(Json(LeaderboardResponse { since, entries }))
}
