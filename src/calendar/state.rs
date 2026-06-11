use std::sync::Arc;

use crate::users::auth::HasJwt;
use crate::users::jwt::JwtService;

use super::config::Config;

#[derive(Clone)]
pub struct State {
    pub db: sqlx::PgPool,
    pub jwt: Arc<JwtService>,
    pub config: Arc<Config>,
}

impl HasJwt for State {
    fn jwt(&self) -> &JwtService {
        &self.jwt
    }
}
