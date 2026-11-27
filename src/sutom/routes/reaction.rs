use axum::{Json, extract::{Path, State}};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::sutom::{domain, repository, state::State as SutomState};
use crate::users::auth::CurrentUser;

#[derive(Deserialize, ToSchema)]
pub struct SetReactionRequest {
    /// One of the allowed emotes, or `null` to remove the reaction.
    pub reaction: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct SetReactionResponse {
    pub reaction: Option<String>,
}

#[utoipa::path(
    put,
    path = "/sutom/day/{date}/reaction",
    tag = "Sutom",
    params(("date" = String, Path, description = "Game date, YYYY-MM-DD")),
    request_body = SetReactionRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Reaction saved", body = SetReactionResponse),
        (status = 400, description = "Unknown emote, or no finished game that day"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn set_reaction(
    State(state): State<SutomState>,
    Path(date): Path<NaiveDate>,
    user: CurrentUser,
    Json(body): Json<SetReactionRequest>,
) -> Result<Json<SetReactionResponse>, AppError> {
    if let Some(reaction) = &body.reaction
        && !domain::REACTIONS.contains(&reaction.as_str())
    {
        return Err(AppError::BadRequest("Réaction inconnue.".into()));
    }

    if !repository::set_reaction(&state.db, user.id, date, body.reaction.as_deref()).await? {
        return Err(AppError::BadRequest("Terminez la partie pour pouvoir y réagir.".into()));
    }
    Ok(Json(SetReactionResponse { reaction: body.reaction }))
}
