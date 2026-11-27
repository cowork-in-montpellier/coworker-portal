use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::lunch::{domain, repository, state::State as LunchState};
use crate::users::auth::CurrentUser;

#[derive(Deserialize, ToSchema)]
pub struct SetIntentionRequest {
    /// Where the member has lunch today, or `null` to take it back.
    pub place_id: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct SetIntentionResponse {
    pub place_id: Option<i64>,
}

#[utoipa::path(
    put,
    path = "/lunch/today/intention",
    tag = "Lunch",
    request_body = SetIntentionRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Today's choice saved, replacing any earlier one", body = SetIntentionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such place"),
    )
)]
pub async fn set_intention(
    State(state): State<LunchState>,
    user: CurrentUser,
    Json(body): Json<SetIntentionRequest>,
) -> Result<Json<SetIntentionResponse>, AppError> {
    if let Some(place_id) = body.place_id
        && repository::is_gamelle(&state.db, place_id).await?.is_none()
    {
        return Err(AppError::NotFound);
    }
    repository::set_intention(&state.db, user.id, domain::today(), body.place_id).await?;
    Ok(Json(SetIntentionResponse { place_id: body.place_id }))
}
