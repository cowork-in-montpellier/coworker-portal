use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub listen_addr: String,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub issuer_address: String,
    pub django_base_url: String,
    pub django_accept_invalid_certs: bool,
    pub guest_user_id: i32,
    pub django_superuser_username: String,
    pub django_superuser_password: String,
    pub unify: UnifyConfig,
    pub voucher_sync_cron: String,
    pub monthly_usage_cron: String,
    pub google_calendar_ical_url: Option<String>,
    pub google_calendar_sync_cron: String,
    pub google_calendar_room_id: i32,
    pub google_caldav_enabled: bool,
    pub google_caldav_email: Option<String>,
    pub google_caldav_password: Option<String>,
    pub google_caldav_calendar_id: Option<String>,
    pub smtp: Option<SmtpConfig>,
    pub app_base_url: String,
}

#[derive(Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
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
            tls_cert_path: std::env::var("TLS_CERT_PATH").ok().map(PathBuf::from),
            tls_key_path: std::env::var("TLS_KEY_PATH").ok().map(PathBuf::from),
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
            voucher_sync_cron: std::env::var("VOUCHER_SYNC_CRON")
                .unwrap_or_else(|_| "0 0 9-19 * * 1-5".into()),
            monthly_usage_cron: std::env::var("MONTHLY_USAGE_CRON")
                .unwrap_or_else(|_| "0 0 9-19 * * *".into()),
            google_calendar_ical_url: std::env::var("GOOGLE_CALENDAR_ICAL_URL").ok(),
            google_calendar_sync_cron: std::env::var("GOOGLE_CALENDAR_SYNC_CRON")
                .unwrap_or_else(|_| "0 */15 * * * *".into()),
            google_calendar_room_id: std::env::var("GOOGLE_CALENDAR_ROOM_ID")
                .unwrap_or_else(|_| "1".into())
                .parse()
                .context("GOOGLE_CALENDAR_ROOM_ID must be a number")?,
            google_caldav_enabled: std::env::var("GOOGLE_CALDAV_ENABLED").as_deref() == Ok("true"),
            google_caldav_email: std::env::var("GOOGLE_CALDAV_EMAIL").ok(),
            google_caldav_password: std::env::var("GOOGLE_CALDAV_PASSWORD").ok(),
            google_caldav_calendar_id: std::env::var("GOOGLE_CALDAV_CALENDAR_ID").ok(),
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
