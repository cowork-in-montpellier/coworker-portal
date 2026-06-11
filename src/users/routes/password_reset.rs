use axum::{Json, extract::State, http::StatusCode};
use rand::{Rng, distributions::Alphanumeric};
use serde::Deserialize;
use serde_json::json;
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::users::email::send_smtp_email;
use crate::users::password::hash_django_password;
use crate::users::state::State as UsersState;

type ApiResult<T> = Result<T, (StatusCode, Json<serde_json::Value>)>;

#[derive(Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(FromRow)]
struct ResetTokenRow {
    user_id: i32,
    expires_at: chrono::DateTime<chrono::Utc>,
    used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[utoipa::path(
    post,
    path = "/forgot-password",
    tag = "Auth",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Reset link sent if email exists"),
        (status = 503, description = "SMTP not configured"),
    )
)]
pub async fn forgot_password(
    State(state): State<UsersState>,
    Json(body): Json<ForgotPasswordRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let smtp = state.config.smtp.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error": "Réinitialisation par email non configurée"})),
    ))?;

    let user_id: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM auth_user WHERE email = $1 AND is_active = true",
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    if let Some(uid) = user_id {
        let _ = sqlx::query(
            "DELETE FROM portal_password_reset_tokens WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(uid)
        .execute(&state.db)
        .await;

        let token = {
            let mut tok = None;
            for _ in 0..5 {
                let candidate: String = rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(64)
                    .map(char::from)
                    .collect();
                let inserted: Option<String> = sqlx::query_scalar(
                    "INSERT INTO portal_password_reset_tokens (user_id, token, expires_at) \
                     VALUES ($1, $2, NOW() + INTERVAL '30 minutes') \
                     ON CONFLICT (token) DO NOTHING RETURNING token",
                )
                .bind(uid)
                .bind(&candidate)
                .fetch_optional(&state.db)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

                if inserted.is_some() {
                    tok = Some(candidate);
                    break;
                }
            }
            tok.ok_or((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?
        };

        let reset_link = format!("{}/reset-password?token={}", state.config.app_base_url, token);
        if let Err(e) = send_smtp_email(smtp, &body.email, "Réinitialisation de votre mot de passe", format!("Bonjour,\n\nCliquez sur le lien suivant pour réinitialiser votre mot de passe :\n{reset_link}\n\nCe lien expire dans 30 minutes.\n\nSi vous n'avez pas fait cette demande, ignorez cet email.")).await {
            tracing::error!(error = %e, "Failed to send reset email");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur lors de l'envoi de l'email"}))));
        }
    }

    Ok(Json(json!({})))
}

#[utoipa::path(
    post,
    path = "/reset-password",
    tag = "Auth",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successfully"),
        (status = 400, description = "Invalid, expired, or already-used token"),
    )
)]
pub async fn reset_password(
    State(state): State<UsersState>,
    Json(body): Json<ResetPasswordRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_as::<_, ResetTokenRow>(
        "SELECT user_id, expires_at, used_at FROM portal_password_reset_tokens WHERE token = $1",
    )
    .bind(&body.token)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?
    .ok_or((StatusCode::BAD_REQUEST, Json(json!({"error": "Token invalide"}))))?;

    if row.used_at.is_some() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Token déjà utilisé"}))));
    }
    if row.expires_at < chrono::Utc::now() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Token expiré"}))));
    }
    if body.new_password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Le mot de passe doit contenir au moins 8 caractères"}))));
    }

    let hashed = hash_django_password(&body.new_password);

    sqlx::query("UPDATE auth_user SET password = $1 WHERE id = $2")
        .bind(&hashed)
        .bind(row.user_id)
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    sqlx::query("UPDATE portal_password_reset_tokens SET used_at = NOW() WHERE token = $1")
        .bind(&body.token)
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    Ok(Json(json!({})))
}
