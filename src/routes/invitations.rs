use axum::{Json, extract::State, http::StatusCode};
use rand::{Rng, distributions::Alphanumeric};
use serde::Deserialize;
use serde_json::json;
use utoipa::ToSchema;

use crate::AppState;
use crate::auth::CurrentUser;
use crate::auth::routes::send_smtp_email;

type ApiResult<T> = Result<T, (StatusCode, Json<serde_json::Value>)>;

#[derive(Deserialize, ToSchema)]
pub struct SendInvitationRequest {
    pub email: String,
}

#[utoipa::path(
    post,
    path = "/invitations",
    tag = "Invitations",
    security(("bearer_auth" = [])),
    request_body = SendInvitationRequest,
    responses(
        (status = 200, description = "Invitation sent"),
        (status = 409, description = "Email already registered"),
        (status = 503, description = "SMTP not configured"),
    )
)]
pub async fn send_invitation(
    State(state): State<AppState>,
    CurrentUser { id: user_id, .. }: CurrentUser,
    Json(body): Json<SendInvitationRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let smtp = state.config.smtp.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error": "Invitations par email non configurées"})),
    ))?;

    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM auth_user WHERE email = $1 AND is_active = true",
    )
    .bind(&body.email)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    if existing.is_some() {
        return Err((StatusCode::CONFLICT, Json(json!({"error": "Un compte avec cet email existe déjà"}))));
    }

    let _ = sqlx::query(
        "DELETE FROM portal_invitation_tokens WHERE email = $1 AND used_at IS NULL",
    )
    .bind(&body.email)
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
                "INSERT INTO portal_invitation_tokens (email, invited_by, token, expires_at) \
                 VALUES ($1, $2, $3, NOW() + INTERVAL '48 hours') \
                 ON CONFLICT (token) DO NOTHING RETURNING token",
            )
            .bind(&body.email)
            .bind(user_id)
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

    let invite_link = format!("{}/accept-invite?token={}", state.config.app_base_url, token);
    if let Err(e) = send_smtp_email(
        smtp,
        &body.email,
        "Invitation à rejoindre Cowork'in Montpellier",
        format!("Bonjour,\n\nVous avez été invité(e) à rejoindre Cowork'in Montpellier.\n\nCliquez sur le lien suivant pour créer votre compte :\n{invite_link}\n\nCe lien expire dans 48 heures.\n\nSi vous n'attendiez pas cette invitation, ignorez cet email."),
    ).await {
        tracing::error!(error = %e, "Failed to send invitation email");
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"Erreur lors de l'envoi de l'email"}))));
    }

    Ok(Json(json!({})))
}
