use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::users::password::verify_django_password;
use crate::users::state::State as UsersState;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(FromRow)]
struct AuthUser {
    id: i32,
    username: String,
    password: String,
    first_name: String,
}

#[utoipa::path(
    post,
    path = "/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "JWT token issued", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
    )
)]
pub async fn login(
    State(state): State<UsersState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    tracing::info!(identifier = &body.username, "Login attempt");
    let user = sqlx::query_as::<_, AuthUser>(
        "SELECT id, username, password, first_name FROM auth_user WHERE (username = $1 OR email = $1) AND is_active = true",
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    if !verify_django_password(&body.password, &user.password) {
        tracing::info!(identifier = &body.username, "Login refused");
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = state
        .jwt
        .generate(user.id, &user.username, &user.first_name)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token }))
}
