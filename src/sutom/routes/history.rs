use axum::{Json, extract::State};
use chrono::NaiveDate;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::sutom::{repository, service, state::State as SutomState};
use crate::users::auth::CurrentUser;

#[derive(Serialize, ToSchema)]
pub struct HistoryEntry {
    pub date: NaiveDate,
    pub puzzle_number: i32,
    pub my_score: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct HistoryResponse {
    pub entries: Vec<HistoryEntry>,
}

#[utoipa::path(
    get,
    path = "/sutom/history",
    tag = "Sutom",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Playable days over the last 30 days and the caller's score on each", body = HistoryResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_history(
    State(state): State<SutomState>,
    user: CurrentUser,
) -> Result<Json<HistoryResponse>, AppError> {
    let today = service::today_paris();
    let since = service::history_window_start(today);
    let dates = repository::list_recent_dates(&state.db, since).await?;

    let mut entries = Vec::with_capacity(dates.len());
    for (date, puzzle_number) in dates {
        let attempt = repository::get_attempt(&state.db, user.id, date).await?;
        entries.push(HistoryEntry { date, puzzle_number, my_score: attempt.and_then(|a| a.score) });
    }

    Ok(Json(HistoryResponse { entries }))
}
