use std::collections::HashMap;

use async_trait::async_trait;

use crate::lunch::ports::{MemberDirectory, MemberName};

pub struct PgMemberDirectory {
    db: sqlx::PgPool,
}

impl PgMemberDirectory {
    pub fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MemberDirectory for PgMemberDirectory {
    async fn names(&self, user_ids: &[i32]) -> anyhow::Result<HashMap<i32, MemberName>> {
        let rows: Vec<(i32, String, String)> =
            sqlx::query_as("SELECT id, first_name, last_name FROM auth_user WHERE id = ANY($1)")
                .bind(user_ids)
                .fetch_all(&self.db)
                .await?;

        Ok(rows
            .into_iter()
            .map(|(id, first_name, last_name)| (id, MemberName { first_name, last_name }))
            .collect())
    }
}
