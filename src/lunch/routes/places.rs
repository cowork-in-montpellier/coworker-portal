use axum::{Json, extract::{Path, State}};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::lunch::domain::{self, PlaceInput};
use crate::lunch::{repository, state::State as LunchState};
use crate::users::auth::CurrentUser;

#[derive(Serialize, ToSchema)]
pub struct PlaceIdResponse {
    pub id: i64,
}

/// Fails unless `id` is a place other than the "J'ai ma gamelle" entry.
pub(super) async fn ensure_regular_place(state: &LunchState, id: i64, gamelle_message: &str) -> Result<(), AppError> {
    match repository::is_gamelle(&state.db, id).await? {
        None => Err(AppError::NotFound),
        Some(true) => Err(AppError::BadRequest(gamelle_message.into())),
        Some(false) => Ok(()),
    }
}

#[utoipa::path(
    post,
    path = "/lunch/places",
    tag = "Lunch",
    request_body = PlaceInput,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Place created", body = PlaceIdResponse),
        (status = 400, description = "Invalid place"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn create_place(
    State(state): State<LunchState>,
    user: CurrentUser,
    Json(body): Json<PlaceInput>,
) -> Result<Json<PlaceIdResponse>, AppError> {
    let place = domain::validate_place(body).map_err(AppError::BadRequest)?;
    let id = repository::create_place(&state.db, &place, user.id).await?;
    Ok(Json(PlaceIdResponse { id }))
}

#[utoipa::path(
    put,
    path = "/lunch/places/{id}",
    tag = "Lunch",
    params(("id" = i64, Path, description = "Place ID")),
    request_body = PlaceInput,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Place updated", body = PlaceIdResponse),
        (status = 400, description = "Invalid place, or the gamelle entry"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such place"),
    )
)]
pub async fn update_place(
    State(state): State<LunchState>,
    Path(id): Path<i64>,
    user: CurrentUser,
    Json(body): Json<PlaceInput>,
) -> Result<Json<PlaceIdResponse>, AppError> {
    ensure_regular_place(&state, id, "La gamelle ne se modifie pas.").await?;
    let place = domain::validate_place(body).map_err(AppError::BadRequest)?;
    if !repository::update_place(&state.db, id, &place, user.id).await? {
        return Err(AppError::NotFound);
    }
    Ok(Json(PlaceIdResponse { id }))
}

#[utoipa::path(
    delete,
    path = "/lunch/places/{id}",
    tag = "Lunch",
    params(("id" = i64, Path, description = "Place ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Place deleted, with its ratings and today's intentions", body = PlaceIdResponse),
        (status = 400, description = "The gamelle entry"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No such place"),
    )
)]
pub async fn delete_place(
    State(state): State<LunchState>,
    Path(id): Path<i64>,
    _user: CurrentUser,
) -> Result<Json<PlaceIdResponse>, AppError> {
    ensure_regular_place(&state, id, "La gamelle ne se supprime pas.").await?;
    if !repository::delete_place(&state.db, id).await? {
        return Err(AppError::NotFound);
    }
    Ok(Json(PlaceIdResponse { id }))
}
