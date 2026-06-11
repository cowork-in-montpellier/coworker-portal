use anyhow::{Context, Result};

#[derive(Clone)]
pub struct Config {
    pub google_calendar_ical_url: Option<String>,
    pub google_calendar_sync_cron: String,
    pub google_calendar_room_id: i32,
    pub google_caldav_enabled: bool,
    pub google_caldav_email: Option<String>,
    pub google_caldav_password: Option<String>,
    pub google_caldav_calendar_id: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
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
        })
    }
}
