use std::collections::HashMap;

use async_trait::async_trait;

use crate::sutom::ports::{PlayerDirectory, PlayerName};

pub struct PgPlayerDirectory {
    db: sqlx::PgPool,
}

impl PgPlayerDirectory {
    pub fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PlayerDirectory for PgPlayerDirectory {
    async fn names(&self, user_ids: &[i32]) -> anyhow::Result<HashMap<i32, PlayerName>> {
        let rows: Vec<(i32, String, String)> =
            sqlx::query_as("SELECT id, first_name, last_name FROM auth_user WHERE id = ANY($1)")
                .bind(user_ids)
                .fetch_all(&self.db)
                .await?;

        Ok(rows
            .into_iter()
            .map(|(id, first_name, last_name)| (id, PlayerName { first_name, last_name }))
            .collect())
    }
}
