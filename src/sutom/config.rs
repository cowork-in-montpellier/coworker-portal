#[derive(Clone)]
pub struct Config {
    pub refresh_cron: String,
    pub dictionary_ttl: std::time::Duration,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            // Shortly after midnight Paris time, so the day's word is warm before the first player.
            refresh_cron: std::env::var("SUTOM_REFRESH_CRON")
                .unwrap_or_else(|_| "0 5 0 * * *".into()),
            // Dictionaries evicted after this long without being read.
            dictionary_ttl: std::time::Duration::from_secs(
                std::env::var("SUTOM_DICTIONARY_TTL_SECS")
                    .ok()
                    .map(|v| v.parse())
                    .transpose()
                    .map_err(|e| anyhow::anyhow!("Invalid SUTOM_DICTIONARY_TTL_SECS: {e}"))?
                    .unwrap_or(24 * 60 * 60),
            ),
        })
    }
}
