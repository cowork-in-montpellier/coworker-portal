use std::sync::Arc;

use crate::users::auth::HasJwt;
use crate::users::jwt::JwtService;

use super::config::Config;
use super::dictionary::DictionaryCache;
use super::ports::PlayerDirectory;

#[derive(Clone)]
pub struct State {
    /// SUTOM's own SQLite database, apart from the billing one.
    pub db: sqlx::SqlitePool,
    pub players: Arc<dyn PlayerDirectory>,
    pub http: reqwest::Client,
    pub jwt: Arc<JwtService>,
    pub config: Arc<Config>,
    pub dictionaries: Arc<DictionaryCache>,
}

impl HasJwt for State {
    fn jwt(&self) -> &JwtService {
        &self.jwt
    }
}
