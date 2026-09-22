use axum::{Json, extract::{Path, State}};
use chrono::NaiveDate;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::sutom::{repository, service, state::State as SutomState};
use crate::users::auth::CurrentUser;

#[derive(Serialize, ToSchema)]
pub struct DayResponse {
    pub date: NaiveDate,
    pub puzzle_number: i32,
    pub word: String,
    pub possible_words: Vec<String>,
    /// The frozen par once the nightly job has computed it, otherwise a live estimate
    /// from whoever has finished the day so far (changes as more players finish).
    pub par: Option<i32>,
    pub my_guesses: Vec<String>,
    pub my_score: Option<i32>,
}

async fn build_day_response(
    state: &SutomState,
    date: NaiveDate,
    user: &CurrentUser,
) -> Result<DayResponse, AppError> {
    let daily = service::ensure_daily_word(state, date).await.map_err(|e| AppError::External(e.to_string()))?;
    let attempt = repository::get_attempt(&state.db, user.id, date).await?;
    let par = service::effective_par(state, date, daily.par).await?;

    Ok(DayResponse {
        date,
        puzzle_number: daily.puzzle_number,
        word: daily.word,
        possible_words: daily.possible_words,
        par,
        my_guesses: attempt.as_ref().map(|a| a.guesses.clone()).unwrap_or_default(),
        my_score: attempt.and_then(|a| a.score),
    })
}

#[utoipa::path(
    get,
    path = "/sutom/today",
    tag = "Sutom",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Today's SUTOM puzzle and the caller's progress on it", body = DayResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_today(
    State(state): State<SutomState>,
    user: CurrentUser,
) -> Result<Json<DayResponse>, AppError> {
    let today = service::today_paris();
    Ok(Json(build_day_response(&state, today, &user).await?))
}

#[utoipa::path(
    get,
    path = "/sutom/day/{date}",
    tag = "Sutom",
    params(("date" = String, Path, description = "Game date, YYYY-MM-DD (last 30 days only)")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "A past SUTOM puzzle and the caller's progress on it", body = DayResponse),
        (status = 400, description = "Date outside the playable window"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_day(
    State(state): State<SutomState>,
    Path(date): Path<NaiveDate>,
    user: CurrentUser,
) -> Result<Json<DayResponse>, AppError> {
    let today = service::today_paris();
    if !service::is_playable(date, today) {
        return Err(AppError::BadRequest("Cette date n'est pas jouable.".into()));
    }
    Ok(Json(build_day_response(&state, date, &user).await?))
}
