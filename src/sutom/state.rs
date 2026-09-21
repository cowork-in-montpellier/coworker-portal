use std::sync::Arc;

use super::config::Config;

#[derive(Clone)]
pub struct State {
    pub db: sqlx::PgPool,
    pub http: reqwest::Client,
    pub config: Arc<Config>,
}
