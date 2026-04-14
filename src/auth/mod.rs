pub mod jwt;
pub mod password;
pub mod routes;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(routes::login))
}

/// Authenticated user extracted from the JWT Bearer token.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: i32,
    pub first_name: String,
    pub django_session: Option<String>,
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Bearer token"))?;

        let claims = state
            .jwt
            .verify(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

        Ok(CurrentUser {
            id: claims.sub,
            first_name: claims.first_name,
            django_session: claims.django_session,
        })
    }
}
