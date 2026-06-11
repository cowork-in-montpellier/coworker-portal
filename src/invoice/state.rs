use std::sync::Arc;

use tokio::sync::RwLock;

use crate::users::auth::HasJwt;
use crate::users::jwt::JwtService;

use super::config::Config;
use super::ports::BillingDirectory;
use super::unify::UnifyClient;

#[derive(Clone)]
pub struct State {
    pub db: sqlx::PgPool,
    pub jwt: Arc<JwtService>,
    pub unify: Arc<dyn UnifyClient>,
    pub billing_directory: Arc<dyn BillingDirectory>,
    /// Cached superuser Django session for invoice PDF proxy.
    /// Acquired at startup; refreshed on failure.
    pub superuser_session: Arc<RwLock<Option<String>>>,
    pub config: Arc<Config>,
}

impl HasJwt for State {
    fn jwt(&self) -> &JwtService {
        &self.jwt
    }
}
