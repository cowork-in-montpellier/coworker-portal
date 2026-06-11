use async_trait::async_trait;

/// Read access to a user's billing address, owned by the `users` module
/// (`billjobs_userprofile`). Implemented by `users::adapters::PgBillingDirectory`.
#[async_trait]
pub trait BillingDirectory: Send + Sync {
    async fn billing_address(&self, user_id: i32) -> anyhow::Result<String>;
}
