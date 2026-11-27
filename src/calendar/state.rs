use std::sync::Arc;

use crate::users::auth::HasJwt;
use crate::users::jwt::JwtService;

use super::config::Config;

#[derive(Clone)]
pub struct State {
    /// The calendar's own SQLite database, apart from the billing one and from SUTOM's.
    pub db: sqlx::SqlitePool,
    pub jwt: Arc<JwtService>,
    pub config: Arc<Config>,
}

impl HasJwt for State {
    fn jwt(&self) -> &JwtService {
        &self.jwt
    }
}
