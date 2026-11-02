use axum::{Json, extract::{Path, State}};
use chrono::NaiveDate;
use serde::Serialize;
use std::cmp::Ordering;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::sutom::{domain, repository, service, state::State as SutomState};
use crate::users::auth::CurrentUser;

#[derive(Serialize, ToSchema)]
pub struct ScoreboardPlayer {
    pub user_id: i32,
    pub first_name: String,
    /// First letter of the player's last name (e.g. disambiguates two "Alice"s).
    pub last_initial: String,
    /// True once this player has solved or exhausted their attempts for the day.
    pub finished: bool,
    /// True once `sequence` is populated. The letter-by-letter grid stays hidden from
    /// the caller until the caller has finished their own game for the day, so nobody
    /// can peek at letter feedback before playing. `score`/`points` are not gated by
    /// this — they're visible to everyone as soon as a player finishes, since a bare
    /// number doesn't leak anything about the word itself.
    pub revealed: bool,
    pub score: Option<i32>,
    pub sequence: Option<Vec<Vec<domain::LetterStatus>>>,
    /// True for whoever finished this day's puzzle earliest, win or lose.
    pub first_to_finish: bool,
    /// True if the word was found less than a minute after the first guess.
    pub fast: bool,
    /// How many days after the puzzle's own day it was finished (0 = on the day). Lets
    /// the UI explain a player's point total precisely.
    pub days_late: i32,
    /// This day's leaderboard points for the player, once revealed.
    pub points: Option<f64>,
}

#[derive(Serialize, ToSchema)]
pub struct ScoreboardResponse {
    pub date: NaiveDate,
    pub players: Vec<ScoreboardPlayer>,
}

#[utoipa::path(
    get,
    path = "/sutom/day/{date}/scoreboard",
    tag = "Sutom",
    params(("date" = String, Path, description = "Game date, YYYY-MM-DD (last 30 days only)")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Who has played this day and, once you've finished it yourself, how they did", body = ScoreboardResponse),
        (status = 400, description = "Date outside the playable window"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_scoreboard(
    State(state): State<SutomState>,
    Path(date): Path<NaiveDate>,
    user: CurrentUser,
) -> Result<Json<ScoreboardResponse>, AppError> {
    let today = service::today_paris();
    if !service::is_playable(date, today) {
        return Err(AppError::BadRequest("Cette date n'est pas jouable.".into()));
    }

    let daily = service::ensure_daily_word(&state, date)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;
    let attempts = repository::list_attempts_for_date(&state.db, date).await?;
    let par = service::effective_par(&state, date, daily.par).await?;

    let viewer_finished = attempts.iter().any(|a| a.user_id == user.id && a.score.is_some());
    let first_finisher_id = attempts
        .iter()
        .filter(|a| a.score.is_some())
        .min_by_key(|a| a.updated_at)
        .map(|a| a.user_id);

    // Ranked by number of guesses, then by points (which also reflect bonuses and
    // lateness), then by whoever finished first. Players still playing sort last.
    let mut scored: Vec<_> = attempts
        .into_iter()
        .map(|a| {
            let is_first = Some(a.user_id) == first_finisher_id;
            let days_late = domain::days_late(date, a.updated_at);
            let fast = a.score.is_some_and(|score| domain::is_fast_finish(a.started_at, a.updated_at, score));
            let points = match (a.score, par) {
                (Some(score), Some(par)) => Some(domain::leaderboard_points(par, score, is_first, fast, days_late)),
                _ => None,
            };
            (a, is_first, fast, days_late, points)
        })
        .collect();

    scored.sort_by(|(a, _, _, _, pa), (b, _, _, _, pb)| match (a.score, b.score) {
        (Some(sa), Some(sb)) => sa
            .cmp(&sb)
            .then_with(|| pb.partial_cmp(pa).unwrap_or(Ordering::Equal))
            .then_with(|| a.updated_at.cmp(&b.updated_at)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.first_name.cmp(&b.first_name),
    });

    let players = scored
        .into_iter()
        .map(|(a, is_first, fast, days_late, points)| {
            let finished = a.score.is_some();
            let revealed = finished && (a.user_id == user.id || viewer_finished);
            let sequence = revealed
                .then(|| a.guesses.iter().map(|g| domain::score_guess(&daily.word, g)).collect());

            ScoreboardPlayer {
                user_id: a.user_id,
                first_name: a.first_name,
                last_initial: domain::last_initial(&a.last_name),
                finished,
                revealed,
                score: a.score,
                sequence,
                first_to_finish: is_first,
                fast,
                days_late,
                points,
            }
        })
        .collect();

    Ok(Json(ScoreboardResponse { date, players }))
}
