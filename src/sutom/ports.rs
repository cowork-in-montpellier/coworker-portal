use std::collections::HashMap;

use async_trait::async_trait;

/// A player's name as shown in the scoreboards.
pub struct PlayerName {
    pub first_name: String,
    pub last_name: String,
}

/// Read access to players' names, owned by the `users` module (`auth_user`), since the
/// SUTOM database only knows players by id. Implemented by `users::adapters::PgPlayerDirectory`.
#[async_trait]
pub trait PlayerDirectory: Send + Sync {
    /// Names of the given players. Ids with no account (deleted since) are left out.
    async fn names(&self, user_ids: &[i32]) -> anyhow::Result<HashMap<i32, PlayerName>>;
}
