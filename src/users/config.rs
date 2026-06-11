use anyhow::{Context, Result};

use super::email::SmtpConfig;

#[derive(Clone)]
pub struct Config {
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub smtp: Option<SmtpConfig>,
    pub app_base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            jwt_secret: std::env::var("JWT_SECRET").context("JWT_SECRET must be set")?,
            jwt_expiry_hours: std::env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".into())
                .parse()
                .context("JWT_EXPIRY_HOURS must be a number")?,
            smtp: match std::env::var("SMTP_HOST") {
                Ok(host) => Some(SmtpConfig {
                    host,
                    port: std::env::var("SMTP_PORT")
                        .unwrap_or_else(|_| "587".into())
                        .parse()
                        .context("SMTP_PORT must be a number")?,
                    username: std::env::var("SMTP_USERNAME")
                        .context("SMTP_USERNAME required when SMTP_HOST is set")?,
                    password: std::env::var("SMTP_PASSWORD")
                        .context("SMTP_PASSWORD required when SMTP_HOST is set")?,
                    from_email: std::env::var("SMTP_FROM_EMAIL")
                        .context("SMTP_FROM_EMAIL required when SMTP_HOST is set")?,
                }),
                Err(_) => None,
            },
            app_base_url: std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
        })
    }
}
