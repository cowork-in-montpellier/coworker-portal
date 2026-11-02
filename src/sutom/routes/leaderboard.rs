use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use axum::{Json, extract::{Query, State}};
use chrono::{Datelike, NaiveDate};
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
    /// Completed months (`YYYY-MM`, from `first_titled_month` on) this player won.
    pub won_months: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct LeaderboardResponse {
    /// The ranked month, `YYYY-MM`.
    pub month: String,
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
    let (month_first, month_last) = domain::month_bounds(month_day);

    // Days before the competition's official start were warm-up/training and never count.
    let from = month_first.max(domain::leaderboard_start_date());
    let to = month_last.min(today);
    let rows = if from <= to {
        repository::list_scored_attempts_between(&state.db, from, to).await?
    } else {
        Vec::new()
    };

    let wins = monthly_wins(&state, today).await?;
    let entries = rank(rows)
        .into_iter()
        .map(|t| LeaderboardEntry {
            won_months: wins.get(&t.user_id).cloned().unwrap_or_default(),
            avg_guesses: t.avg_guesses(),
            user_id: t.user_id,
            last_initial: domain::last_initial(&t.last_name),
            first_name: t.first_name,
            points: t.points,
            games_played: t.games_played,
        })
        .collect();

    Ok(Json(LeaderboardResponse { month: month_first.format("%Y-%m").to_string(), entries }))
}

struct Totals {
    user_id: i32,
    first_name: String,
    last_name: String,
    points: f64,
    games_played: i32,
    score_sum: i32,
}

impl Totals {
    fn avg_guesses(&self) -> f64 {
        self.score_sum as f64 / self.games_played as f64
    }
}

/// Sums each player's points over `rows`, best first: most points, then the lowest
/// average number of guesses.
fn rank(rows: Vec<repository::ScoreRow>) -> Vec<Totals> {
    let mut totals: HashMap<i32, Totals> = HashMap::new();
    for row in rows {
        let days_late = domain::days_late(row.game_date, row.updated_at);
        let fast = domain::is_fast_finish(row.started_at, row.updated_at, row.score);
        let points = domain::leaderboard_points(row.par, row.score, row.is_first, fast, days_late);
        let entry = totals.entry(row.user_id).or_insert_with(|| Totals {
            user_id: row.user_id,
            first_name: row.first_name,
            last_name: row.last_name,
            points: 0.0,
            games_played: 0,
            score_sum: 0,
        });
        entry.points += points;
        entry.games_played += 1;
        entry.score_sum += row.score;
    }

    let mut ranked: Vec<Totals> = totals.into_values().collect();
    ranked.sort_by(|a, b| {
        b.points
            .partial_cmp(&a.points)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.avg_guesses().partial_cmp(&b.avg_guesses()).unwrap_or(Ordering::Equal))
    });
    ranked
}

/// The winner of every completed month since `first_titled_month`, as a map from user
/// to the months they won.
async fn monthly_wins(state: &SutomState, today: NaiveDate) -> Result<HashMap<i32, Vec<String>>, AppError> {
    let (current_month_first, _) = domain::month_bounds(today);
    let first = domain::first_titled_month();
    let mut wins: HashMap<i32, Vec<String>> = HashMap::new();
    if first >= current_month_first {
        return Ok(wins);
    }
    let last = current_month_first.pred_opt().expect("day before a month start exists");

    let mut by_month: BTreeMap<(i32, u32), Vec<repository::ScoreRow>> = BTreeMap::new();
    for row in repository::list_scored_attempts_between(&state.db, first, last).await? {
        by_month.entry((row.game_date.year(), row.game_date.month())).or_default().push(row);
    }
    for ((year, month), rows) in by_month {
        if let Some(winner) = rank(rows).into_iter().next() {
            wins.entry(winner.user_id).or_default().push(format!("{year:04}-{month:02}"));
        }
    }
    Ok(wins)
}
