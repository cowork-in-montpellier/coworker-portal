use anyhow::{Context, Result};
use std::path::PathBuf;

/// Cross-cutting infrastructure configuration shared by all modules.
#[derive(Clone)]
pub struct InfraConfig {
    pub database_url: String,
    pub listen_addr: String,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
}

impl InfraConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            listen_addr: std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into()),
            tls_cert_path: std::env::var("TLS_CERT_PATH").ok().map(PathBuf::from),
            tls_key_path: std::env::var("TLS_KEY_PATH").ok().map(PathBuf::from),
        })
    }
}
