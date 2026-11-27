#[derive(Clone)]
pub struct Config {
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            // Pause-déj's own SQLite file, created on first start.
            database_url: std::env::var("LUNCH_DATABASE_URL").unwrap_or_else(|_| "sqlite:data/lunch.db".into()),
        })
    }
}
