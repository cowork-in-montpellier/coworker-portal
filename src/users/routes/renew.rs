use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::users::state::State as UsersState;

#[derive(Deserialize, ToSchema)]
pub struct RenewRequest {
    pub refresh_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct RenewResponse {
    pub token: String,
    pub refresh_token: String,
}

#[derive(FromRow)]
struct ActiveUser {
    username: String,
    first_name: String,
}

#[utoipa::path(
    post,
    path = "/renew",
    tag = "Auth",
    request_body = RenewRequest,
    responses(
        (status = 200, description = "New token pair issued", body = RenewResponse),
        (status = 401, description = "Invalid or expired refresh token"),
    )
)]
pub async fn renew(
    State(state): State<UsersState>,
    Json(body): Json<RenewRequest>,
) -> Result<Json<RenewResponse>, StatusCode> {
    let claims = state
        .jwt
        .verify(&body.refresh_token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if claims.token_type != "refresh" {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user = sqlx::query_as::<_, ActiveUser>(
        "SELECT username, first_name FROM auth_user WHERE id = $1 AND is_active = true",
    )
    .bind(claims.sub)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    if user.username != claims.username {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = state
        .jwt
        .generate(claims.sub, &user.username, &user.first_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let refresh_token = state
        .jwt
        .generate_refresh(claims.sub, &user.username, &user.first_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RenewResponse { token, refresh_token }))
}
