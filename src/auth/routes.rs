use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::AppState;
use super::password::verify_django_password;

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
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    tracing::info!(username = &body.username, "login");
    let user = sqlx::query_as::<_, AuthUser>(
        "SELECT id, password, first_name FROM auth_user WHERE username = $1 AND is_active = true",
    )
    .bind(&body.username)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    if !verify_django_password(&body.password, &user.password) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let django_session = acquire_django_session(
        &state.config.django_base_url,
        state.config.django_accept_invalid_certs,
        &body.username,
        &body.password,
    )
    .await
    .inspect_err(|e| tracing::warn!(error = %e, "Django session acquisition failed — invoice PDF will be unavailable"))
    .ok();

    let token = state
        .jwt
        .generate(user.id, &body.username, &user.first_name, django_session)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token }))
}

/// Authenticates against Django's login page and returns the `sessionid` cookie value.
pub async fn acquire_django_session(
    base_url: &str,
    accept_invalid_certs: bool,
    username: &str,
    password: &str,
) -> anyhow::Result<String> {
    tracing::info!(base_url, username, accept_invalid_certs, "Django: starting session acquisition");

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(accept_invalid_certs)
        .build()?;

    let login_url = format!("{}/admin/login/", base_url);
    tracing::debug!(url = %login_url, "Django: GET login page");

    let get_res = client.get(&login_url).send().await
        .inspect_err(|e| tracing::error!(error = %e, url = %login_url, "Django: GET login page failed"))?;

    tracing::debug!(status = %get_res.status(), "Django: GET login page response");
    tracing::debug!(
        set_cookie = ?get_res.headers().get_all(reqwest::header::SET_COOKIE).iter().collect::<Vec<_>>(),
        "Django: GET Set-Cookie headers"
    );

    let csrf = extract_cookie(get_res.headers(), "csrftoken")
        .unwrap_or_default();
    tracing::debug!(csrf_found = !csrf.is_empty(), "Django: CSRF token extracted");

    if csrf.is_empty() {
        tracing::warn!("Django: csrftoken not found in GET response — POST may be rejected");
    }

    tracing::debug!(url = %login_url, username, "Django: POST credentials");

    let post_res: reqwest::Response = client
        .post(&login_url)
        .header("Cookie", format!("csrftoken={}", csrf))
        .header("Referer", &login_url)
        .form(&[
            ("username", username),
            ("password", password),
            ("csrfmiddlewaretoken", csrf.as_str()),
        ])
        .send()
        .await
        .inspect_err(|e| tracing::error!(error = %e, "Django: POST credentials failed"))?;

    tracing::debug!(status = %post_res.status(), "Django: POST response");
    tracing::debug!(
        set_cookie = ?post_res.headers().get_all(reqwest::header::SET_COOKIE).iter().collect::<Vec<_>>(),
        "Django: POST Set-Cookie headers"
    );

    let session = extract_cookie(post_res.headers(), "sessionid");
    match &session {
        Some(_) => tracing::info!(username, "Django: session acquired successfully"),
        None => tracing::warn!(
            username,
            post_status = %post_res.status(),
            "Django: sessionid not found in POST response — credentials may be wrong or Django rejected the login"
        ),
    }

    session.ok_or_else(|| anyhow::anyhow!("sessionid not found in Django response"))
}

fn extract_cookie(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    let prefix = format!("{}=", name);
    headers
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .find_map(|hv| {
            hv.to_str().ok().and_then(|s| {
                s.split(';')
                    .next()
                    .and_then(|part| part.trim().strip_prefix(&prefix).map(str::to_string))
            })
        })
}
