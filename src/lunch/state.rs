use std::sync::Arc;

use crate::users::auth::HasJwt;
use crate::users::jwt::JwtService;

use super::ports::MemberDirectory;

#[derive(Clone)]
pub struct State {
    /// Pause-déj's own SQLite database, apart from the billing one.
    pub db: sqlx::SqlitePool,
    pub members: Arc<dyn MemberDirectory>,
    pub jwt: Arc<JwtService>,
}

impl HasJwt for State {
    fn jwt(&self) -> &JwtService {
        &self.jwt
    }
}
