use std::sync::Arc;

use super::auth::HasJwt;
use super::config::Config;
use super::jwt::JwtService;

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
