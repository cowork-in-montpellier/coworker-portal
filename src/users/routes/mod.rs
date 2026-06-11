pub mod invite;
pub mod login;
pub mod password_reset;
pub mod profile;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::users::state::State;

/// Routes mounted under `/api/auth`.
pub fn auth_router() -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(login::login))
        .routes(routes!(password_reset::forgot_password))
        .routes(routes!(password_reset::reset_password))
        .routes(routes!(invite::accept_invite))
}

/// Routes mounted under `/api`.
pub fn router() -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(profile::get_profile, profile::update_profile))
        .routes(routes!(profile::change_password))
        .routes(routes!(invite::send_invitation))
}
