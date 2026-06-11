use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        crate::users::routes::login::LoginRequest,
        crate::users::routes::login::LoginResponse,
        crate::users::routes::password_reset::ForgotPasswordRequest,
        crate::users::routes::password_reset::ResetPasswordRequest,
        crate::users::routes::invite::AcceptInviteRequest,
        crate::users::routes::invite::SendInvitationRequest,
        crate::users::routes::profile::ProfileResponse,
        crate::users::routes::profile::UpdateProfileRequest,
        crate::users::routes::profile::ChangePasswordRequest,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication"),
        (name = "Profile", description = "User profile management"),
        (name = "Invitations", description = "Member invitation management"),
    )
)]
pub struct ApiDoc;
