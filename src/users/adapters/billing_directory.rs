use async_trait::async_trait;

use crate::invoice::ports::BillingDirectory;

pub struct PgBillingDirectory {
    db: sqlx::PgPool,
}

impl PgBillingDirectory {
    pub fn new(db: sqlx::PgPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BillingDirectory for PgBillingDirectory {
    async fn billing_address(&self, user_id: i32) -> anyhow::Result<String> {
        let address: Option<String> =
            sqlx::query_scalar::<_, String>("SELECT billing_address FROM billjobs_userprofile WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.db)
                .await?;

        Ok(address.unwrap_or_default())
    }
}
