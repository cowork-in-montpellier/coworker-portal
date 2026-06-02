mod google_calendar_sync;
mod monthly_usage;
mod voucher_sync;

use chrono_tz::Europe::Paris;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::AppState;

/// Spawn all background tasks. Returns immediately; tasks run on the Tokio runtime.
pub async fn start(
    state: AppState,
    voucher_sync_cron: &str,
    monthly_usage_cron: &str,
    google_calendar_ical_url: Option<&str>,
    google_calendar_sync_cron: &str,
    google_calendar_room_id: i32,
) -> anyhow::Result<()> {
    voucher_sync_cron.parse::<croner::Cron>()
        .map_err(|e| anyhow::anyhow!("Invalid VOUCHER_SYNC_CRON {:?}: {}", voucher_sync_cron, e))?;
    monthly_usage_cron.parse::<croner::Cron>()
        .map_err(|e| anyhow::anyhow!("Invalid MONTHLY_USAGE_CRON {:?}: {}", monthly_usage_cron, e))?;
    if google_calendar_ical_url.is_some() {
        google_calendar_sync_cron.parse::<croner::Cron>()
            .map_err(|e| anyhow::anyhow!("Invalid GOOGLE_CALENDAR_SYNC_CRON {:?}: {}", google_calendar_sync_cron, e))?;
    }

    let scheduler = JobScheduler::new().await?;

    // Voucher sync: Mon–Fri, every hour 09:00–19:00 (Paris).
    let s = state.clone();
    let cron = voucher_sync_cron.to_string();
    scheduler.add(Job::new_async_tz(&cron, Paris, move |_id, _sched| {
        let s = s.clone();
        Box::pin(async move { voucher_sync::run(&s).await })
    })?).await?;

    // Monthly usage diary: every day at 23:00 (Paris).
    let s = state.clone();
    let cron = monthly_usage_cron.to_string();
    scheduler.add(Job::new_async_tz(&cron, Paris, move |_id, _sched| {
        let s = s.clone();
        Box::pin(async move { monthly_usage::run(&s).await })
    })?).await?;

    // Google Calendar sync: optional, runs on startup then on schedule.
    if let Some(url) = google_calendar_ical_url {
        let url = url.to_string();

        let s = state.clone();
        let url_clone = url.clone();
        tokio::spawn(async move {
            google_calendar_sync::run(&s, &url_clone, google_calendar_room_id).await;
        });

        let s = state.clone();
        let cron = google_calendar_sync_cron.to_string();
        scheduler.add(Job::new_async_tz(&cron, Paris, move |_id, _sched| {
            let s = s.clone();
            let url = url.clone();
            Box::pin(async move { google_calendar_sync::run(&s, &url, google_calendar_room_id).await })
        })?).await?;

        tracing::info!(
            google_calendar_sync_cron,
            room_id = google_calendar_room_id,
            "Google Calendar sync scheduled"
        );
    }

    scheduler.start().await?;
    tracing::info!(
        voucher_sync_cron,
        monthly_usage_cron,
        timezone = "Europe/Paris",
        "Scheduler started"
    );
    Ok(())
}