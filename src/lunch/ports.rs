use std::collections::HashMap;

use async_trait::async_trait;

/// A member's name, as shown next to where they have lunch.
pub struct MemberName {
    pub first_name: String,
    pub last_name: String,
}

/// Read access to members' names, owned by the `users` module (`auth_user`), since the
/// Pause-déj database only knows members by id. Implemented by `users::adapters::PgMemberDirectory`.
#[async_trait]
pub trait MemberDirectory: Send + Sync {
    /// Names of the given members. Ids with no account (deleted since) are left out.
    async fn names(&self, user_ids: &[i32]) -> anyhow::Result<HashMap<i32, MemberName>>;
}
