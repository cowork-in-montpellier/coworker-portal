use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::users::auth::CurrentUser;
use crate::users::password::{hash_django_password, verify_django_password};
use crate::users::state::State as UsersState;

#[derive(Serialize, ToSchema)]
pub struct ProfileResponse {
    pub id: i32,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub billing_address: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub billing_address: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(sqlx::FromRow)]
struct ProfileRow {
    id: i32,
    username: String,
    first_name: String,
    last_name: String,
    email: String,
    billing_address: Option<String>,
}

async fn fetch_profile(db: &sqlx::PgPool, user_id: i32) -> Result<ProfileResponse, AppError> {
    let row = sqlx::query_as::<_, ProfileRow>(
        r#"
        SELECT u.id, u.username, u.first_name, u.last_name, u.email,
               p.billing_address
        FROM auth_user u
        LEFT JOIN billjobs_userprofile p ON p.user_id = u.id
        WHERE u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(ProfileResponse {
        id: row.id,
        username: row.username,
        first_name: row.first_name,
        last_name: row.last_name,
        email: row.email,
        billing_address: row.billing_address.unwrap_or_default(),
    })
}

#[utoipa::path(
    get,
    path = "/profile",
    tag = "Profile",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user profile", body = ProfileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
    )
)]
pub async fn get_profile(
    State(state): State<UsersState>,
    user: CurrentUser,
) -> Result<Json<ProfileResponse>, AppError> {
    Ok(Json(fetch_profile(&state.db, user.id).await?))
}

#[utoipa::path(
    put,
    path = "/profile",
    tag = "Profile",
    security(("bearer_auth" = [])),
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Updated profile", body = ProfileResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn update_profile(
    State(state): State<UsersState>,
    user: CurrentUser,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, AppError> {
    if body.first_name.trim().is_empty()
        || body.last_name.trim().is_empty()
        || body.email.trim().is_empty()
    {
        return Err(AppError::BadRequest("All fields are required".into()));
    }

    sqlx::query(
        "UPDATE auth_user SET first_name = $1, last_name = $2, email = $3 WHERE id = $4",
    )
    .bind(body.first_name.trim())
    .bind(body.last_name.trim())
    .bind(body.email.trim())
    .bind(user.id)
    .execute(&state.db)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO billjobs_userprofile (user_id, billing_address)
        VALUES ($1, $2)
        ON CONFLICT (user_id) DO UPDATE SET billing_address = EXCLUDED.billing_address
        "#,
    )
    .bind(user.id)
    .bind(body.billing_address.trim())
    .execute(&state.db)
    .await?;

    Ok(Json(fetch_profile(&state.db, user.id).await?))
}

#[utoipa::path(
    put,
    path = "/profile/password",
    tag = "Profile",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed"),
        (status = 400, description = "Current password incorrect or new password too short"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn change_password(
    State(state): State<UsersState>,
    user: CurrentUser,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "Le nouveau mot de passe doit comporter au moins 8 caractères".into(),
        ));
    }

    let stored: String = sqlx::query_scalar("SELECT password FROM auth_user WHERE id = $1")
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    if !verify_django_password(&body.current_password, &stored) {
        return Err(AppError::BadRequest("Mot de passe actuel incorrect".into()));
    }

    let new_hash = hash_django_password(&body.new_password);

    sqlx::query("UPDATE auth_user SET password = $1 WHERE id = $2")
        .bind(&new_hash)
        .bind(user.id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({})))
}
