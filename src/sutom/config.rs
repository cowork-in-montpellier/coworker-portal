#[derive(Clone)]
pub struct Config {
    pub refresh_cron: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            // Shortly after midnight Paris time, so the day's word is warm before the first player.
            refresh_cron: std::env::var("SUTOM_REFRESH_CRON")
                .unwrap_or_else(|_| "0 5 0 * * *".into()),
        })
    }
}
