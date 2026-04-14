use anyhow::{Context, Result};

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub listen_addr: String,
    pub issuer_address: String,
    pub django_base_url: String,
    pub django_accept_invalid_certs: bool,
    pub guest_user_id: i32,
    pub django_superuser_username: String,
    pub django_superuser_password: String,
    pub unify: UnifyConfig,
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
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            jwt_secret: std::env::var("JWT_SECRET").context("JWT_SECRET must be set")?,
            jwt_expiry_hours: std::env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".into())
                .parse()
                .context("JWT_EXPIRY_HOURS must be a number")?,
            listen_addr: std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into()),
            issuer_address: std::env::var("BILL_ISSUER_ADDRESS")
                .unwrap_or_else(|_| "Coworking Space\n1 rue de la Paix\n75001 Paris".into()),
            django_base_url: std::env::var("DJANGO_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8000".into()),
            django_accept_invalid_certs: std::env::var("DJANGO_ACCEPT_INVALID_CERTS").as_deref()
                == Ok("true"),
            guest_user_id: std::env::var("GUEST_USER_ID")
                .unwrap_or_else(|_| "1".into())
                .parse()
                .context("GUEST_USER_ID must be a number")?,
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
