use axum::{Json, extract::{Path, State}};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::sutom::{domain, repository, service, state::State as SutomState};
use crate::users::auth::CurrentUser;

#[derive(Deserialize, ToSchema)]
pub struct SubmitGuessesRequest {
    pub guesses: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SubmitGuessesResponse {
    pub score: Option<i32>,
}

#[utoipa::path(
    put,
    path = "/sutom/day/{date}/attempt",
    tag = "Sutom",
    params(("date" = String, Path, description = "Game date, YYYY-MM-DD (last 30 days only)")),
    request_body = SubmitGuessesRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Updated attempt", body = SubmitGuessesResponse),
        (status = 400, description = "Invalid guess, finished game, or date outside the playable window"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn submit_guesses(
    State(state): State<SutomState>,
    Path(date): Path<NaiveDate>,
    user: CurrentUser,
    Json(body): Json<SubmitGuessesRequest>,
) -> Result<Json<SubmitGuessesResponse>, AppError> {
    let today = service::today_paris();
    if !service::is_playable(date, today) {
        return Err(AppError::BadRequest("Cette date n'est pas jouable.".into()));
    }

    if body.guesses.len() > domain::MAX_ATTEMPTS {
        return Err(AppError::BadRequest("Trop de tentatives.".into()));
    }

    if let Some(existing) = repository::get_attempt(&state.db, user.id, date).await? {
        if existing.score.is_some() {
            return Err(AppError::BadRequest("Cette partie est déjà terminée.".into()));
        }
    }

    let daily = service::ensure_daily_word(&state, date)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;
    let possible_words = service::possible_words(&state, &daily.word)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    let cleaned: Vec<String> = body.guesses.iter().map(|g| domain::clean_word(g)).collect();
    for guess in &cleaned {
        domain::validate_guess(&daily.word, &possible_words, guess).map_err(AppError::BadRequest)?;
    }

    let score = domain::score_for(&daily.word, &cleaned);
    repository::upsert_attempt(&state.db, user.id, date, &cleaned, score).await?;

    Ok(Json(SubmitGuessesResponse { score }))
}
