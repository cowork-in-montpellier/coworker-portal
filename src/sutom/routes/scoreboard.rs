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
    /// True if this was a later catch-up play rather than played on the day itself —
    /// halves the points. Lets the UI explain a player's point total precisely.
    pub is_catchup: bool,
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

    // Compute each player's points up front so the list can be ranked by them — points,
    // not raw score, is the true ranking (a first-to-finish bonus or a catch-up discount
    // can reorder two equal scores) — ties broken by whoever finished earlier. Players
    // still playing have no points yet and always sort after everyone who has finished.
    let mut scored: Vec<_> = attempts
        .into_iter()
        .map(|a| {
            let is_first = Some(a.user_id) == first_finisher_id;
            let is_catchup = domain::is_catchup_play(date, a.updated_at);
            let points = match (a.score, par) {
                (Some(score), Some(par)) => Some(domain::leaderboard_points(par, score, is_first, is_catchup)),
                _ => None,
            };
            (a, is_first, is_catchup, points)
        })
        .collect();

    scored.sort_by(|(a, _, _, pa), (b, _, _, pb)| match (pa, pb) {
        (Some(pa), Some(pb)) => pb
            .partial_cmp(pa)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.updated_at.cmp(&b.updated_at)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.first_name.cmp(&b.first_name),
    });

    let players = scored
        .into_iter()
        .map(|(a, is_first, is_catchup, points)| {
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
                is_catchup,
                points,
            }
        })
        .collect();

    Ok(Json(ScoreboardResponse { date, players }))
}
