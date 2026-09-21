use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::sutom::{service, state::State as SutomState};

#[derive(Serialize, ToSchema)]
pub struct TodayResponse {
    pub date: chrono::NaiveDate,
    pub word: String,
    pub possible_words: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/sutom/today",
    tag = "Sutom",
    responses(
        (status = 200, description = "Today's SUTOM word and its playable dictionary", body = TodayResponse),
    )
)]
pub async fn get_today(State(state): State<SutomState>) -> Result<Json<TodayResponse>, AppError> {
    let today = service::today_paris();
    let daily = service::ensure_daily_word(&state, today)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    Ok(Json(TodayResponse { date: today, word: daily.word, possible_words: daily.possible_words }))
}
