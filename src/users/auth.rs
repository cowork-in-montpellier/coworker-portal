use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
};

use super::jwt::JwtService;

/// Implemented by every module's state that needs to authenticate requests via
/// the [`CurrentUser`] extractor.
pub trait HasJwt {
    fn jwt(&self) -> &JwtService;
}

/// Authenticated user extracted from the JWT Bearer token.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: i32,
    pub first_name: String,
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: HasJwt + Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Bearer token"))?;

        let claims = state
            .jwt()
            .verify(token)
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

        Ok(CurrentUser {
            id: claims.sub,
            first_name: claims.first_name,
        })
    }
}
