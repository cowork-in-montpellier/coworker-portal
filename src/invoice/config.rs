use anyhow::{Context, Result};

#[derive(Clone)]
pub struct Config {
    pub issuer_address: String,
    pub guest_user_id: i32,
    pub unify: UnifyConfig,
    pub voucher_sync_cron: String,
    pub monthly_usage_cron: String,
    pub django_base_url: String,
    pub django_accept_invalid_certs: bool,
    pub django_superuser_username: String,
    pub django_superuser_password: String,
}

#[derive(Clone)]
pub struct UnifyConfig {
    pub mode: UnifyMode,
    pub base_url: String,
    pub site: String,
    pub username: String,
    pub password: String,
    pub accept_invalid_certs: bool,
}

#[derive(Clone, PartialEq)]
pub enum UnifyMode {
    Mock,
    Real,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            issuer_address: std::env::var("BILL_ISSUER_ADDRESS")
                .unwrap_or_else(|_| "Coworking Space\n1 rue de la Paix\n75001 Paris".into()),
            guest_user_id: std::env::var("GUEST_USER_ID")
                .unwrap_or_else(|_| "1".into())
                .parse()
                .context("GUEST_USER_ID must be a number")?,
            voucher_sync_cron: std::env::var("VOUCHER_SYNC_CRON")
                .unwrap_or_else(|_| "0 0 9-19 * * 1-5".into()),
            monthly_usage_cron: std::env::var("MONTHLY_USAGE_CRON")
                .unwrap_or_else(|_| "0 0 9-19 * * *".into()),
            django_base_url: std::env::var("DJANGO_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8000".into()),
            django_accept_invalid_certs: std::env::var("DJANGO_ACCEPT_INVALID_CERTS").as_deref()
                == Ok("true"),
            django_superuser_username: std::env::var("DJANGO_SUPERUSER_USERNAME")
                .unwrap_or_default(),
            django_superuser_password: std::env::var("DJANGO_SUPERUSER_PASSWORD")
                .unwrap_or_default(),
            unify: UnifyConfig {
                mode: if std::env::var("UNIFY_MOCK").as_deref() == Ok("true") {
                    UnifyMode::Mock
                } else {
                    UnifyMode::Real
                },
                base_url: std::env::var("UNIFY_BASE_URL")
                    .unwrap_or_else(|_| "https://192.168.1.1:8443".into()),
                site: std::env::var("UNIFY_SITE").unwrap_or_else(|_| "default".into()),
                username: std::env::var("UNIFY_USERNAME").unwrap_or_default(),
                password: std::env::var("UNIFY_PASSWORD").unwrap_or_default(),
                accept_invalid_certs: std::env::var("UNIFY_ACCEPT_INVALID_CERTS").as_deref()
                    == Ok("true"),
            },
        })
    }
}
