use axum::{body::Body, http::header, response::Response};

use crate::error::AppError;

use super::state::State;

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
        Some(session) => tracing::info!(username, session, "Django: session acquired successfully"),
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

/// Proxy a Django invoice PDF using the cached superuser session.
/// Shared by both authenticated (`bill_pdf`) and guest (`guest_bill_pdf`) routes.
pub async fn proxy_bill_pdf(state: &State, bill_id: i32) -> Result<Response, AppError> {
    let session = {
        let guard = state.superuser_session.read().await;
        guard.clone()
    };

    let session = session.ok_or_else(|| {
        AppError::External("No superuser Django session available — configure DJANGO_SUPERUSER_USERNAME/PASSWORD".into())
    })?;

    match fetch_django_pdf(state, bill_id, &session).await {
        Ok(response) => Ok(response),
        Err(_) => {
            // Session may have expired — try to re-acquire once
            tracing::warn!("Guest PDF: Django returned error, attempting session refresh");
            let new_session = acquire_django_session(
                &state.config.django_base_url,
                state.config.django_accept_invalid_certs,
                &state.config.django_superuser_username,
                &state.config.django_superuser_password,
            )
            .await
            .map_err(|e| AppError::External(format!("Session refresh failed: {e}")))?;

            {
                let mut guard = state.superuser_session.write().await;
                *guard = Some(new_session.clone());
            }

            fetch_django_pdf(state, bill_id, &new_session).await
        }
    }
}

async fn fetch_django_pdf(state: &State, bill_id: i32, session: &str) -> Result<Response, AppError> {
    let url = format!(
        "{}/billjobs/generate_pdf/{}",
        state.config.django_base_url, bill_id
    );

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(state.config.django_accept_invalid_certs)
        .build()
        .map_err(|e| AppError::External(e.to_string()))?;

    let res = client
        .get(&url)
        .header("Cookie", format!("sessionid={}", session))
        .send()
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    if !res.status().is_success() {
        return Err(AppError::External(format!("Django returned {}", res.status())));
    }

    let content_disposition = res.headers().get(header::CONTENT_DISPOSITION).cloned();
    let bytes = res.bytes().await.map_err(|e| AppError::External(e.to_string()))?;

    let mut builder = Response::builder().header(header::CONTENT_TYPE, "application/pdf");
    if let Some(cd) = content_disposition {
        builder = builder.header(header::CONTENT_DISPOSITION, cd);
    }

    Ok(builder.body(Body::from(bytes)).unwrap())
}
