use axum::{Json, extract::State, http::StatusCode};
use rand::{Rng, distributions::Alphanumeric};
use serde::Deserialize;
use serde_json::json;
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::users::auth::CurrentUser;
use crate::users::email::send_smtp_email;
use crate::users::password::hash_django_password;
use crate::users::state::State as UsersState;

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
    State(state): State<UsersState>,
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

#[derive(Deserialize, ToSchema)]
pub struct AcceptInviteRequest {
    pub token: String,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub password: String,
}

#[derive(FromRow)]
struct InviteTokenRow {
    email: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    used_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[utoipa::path(
    post,
    path = "/accept-invite",
    tag = "Invitations",
    request_body = AcceptInviteRequest,
    responses(
        (status = 200, description = "Account created successfully"),
        (status = 400, description = "Invalid token or validation error"),
    )
)]
pub async fn accept_invite(
    State(state): State<UsersState>,
    Json(body): Json<AcceptInviteRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_as::<_, InviteTokenRow>(
        "SELECT email, expires_at, used_at FROM portal_invitation_tokens WHERE token = $1",
    )
    .bind(&body.token)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?
    .ok_or((StatusCode::BAD_REQUEST, Json(json!({"error": "Invitation invalide"}))))?;

    if row.used_at.is_some() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invitation déjà utilisée"}))));
    }
    if row.expires_at < chrono::Utc::now() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invitation expirée"}))));
    }

    let username_taken: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM auth_user WHERE username = $1",
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    if username_taken.is_some() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Nom d'utilisateur déjà utilisé"}))));
    }

    if body.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Le mot de passe doit contenir au moins 8 caractères"}))));
    }

    let hashed = hash_django_password(&body.password);

    let user_id: i32 = sqlx::query_scalar(
        "INSERT INTO auth_user \
         (password, last_login, is_superuser, username, first_name, last_name, email, is_staff, is_active, date_joined) \
         VALUES ($1, NULL, false, $2, $3, $4, $5, false, true, NOW()) \
         RETURNING id",
    )
    .bind(&hashed)
    .bind(&body.username)
    .bind(&body.first_name)
    .bind(&body.last_name)
    .bind(&row.email)
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    sqlx::query("INSERT INTO billjobs_userprofile (user_id, billing_address) VALUES ($1, '')")
        .bind(user_id)
        .execute(&state.db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    let _ = sqlx::query(
        "UPDATE portal_invitation_tokens SET used_at = NOW() WHERE email = $1 AND used_at IS NULL",
    )
    .bind(&row.email)
    .execute(&state.db)
    .await;

    Ok(Json(json!({})))
}
