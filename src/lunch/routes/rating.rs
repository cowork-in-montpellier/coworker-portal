use axum::{Json, extract::{Path, State}};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::lunch::{repository, state::State as LunchState};
use crate::users::auth::CurrentUser;

use super::places::ensure_regular_place;

#[derive(Deserialize, ToSchema)]
pub struct SetRatingRequest {
    /// From 1 to 5 stars, or `null` to remove the rating.
    pub stars: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct SetRatingResponse {
    pub stars: Option<i64>,
}

#[utoipa::path(
    put,
    path = "/lunch/places/{id}/rating",
    tag = "Lunch",
    params(("id" = i64, Path, description = "Place ID")),
    request_body = SetRatingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Rating saved", body = SetRatingResponse),
        (status = 400, description = "Not between 1 and 5, or the gamelle entry"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such place"),
    )
)]
pub async fn set_rating(
    State(state): State<LunchState>,
    Path(id): Path<i64>,
    user: CurrentUser,
    Json(body): Json<SetRatingRequest>,
) -> Result<Json<SetRatingResponse>, AppError> {
    if body.stars.is_some_and(|s| !(1..=5).contains(&s)) {
        return Err(AppError::BadRequest("Une note va de 1 à 5 étoiles.".into()));
    }
    ensure_regular_place(&state, id, "Ta gamelle est forcément délicieuse, pas besoin de la noter.").await?;
    repository::set_rating(&state.db, id, user.id, body.stars).await?;
    Ok(Json(SetRatingResponse { stars: body.stars }))
}
