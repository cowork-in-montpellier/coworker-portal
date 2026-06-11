mod google_calendar_sync;

use chrono_tz::Europe::Paris;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::calendar::state::State;

/// Register the calendar module's background tasks on the shared scheduler.
pub async fn register(scheduler: &JobScheduler, state: State) -> anyhow::Result<()> {
    let google_calendar_ical_url = state.config.google_calendar_ical_url.clone();
    let google_calendar_sync_cron = state.config.google_calendar_sync_cron.clone();
    let google_calendar_room_id = state.config.google_calendar_room_id;

    let Some(url) = google_calendar_ical_url else {
        return Ok(());
    };

    google_calendar_sync_cron
        .parse::<croner::Cron>()
        .map_err(|e| {
            anyhow::anyhow!(
                "Invalid GOOGLE_CALENDAR_SYNC_CRON {:?}: {}",
                google_calendar_sync_cron,
                e
            )
        })?;

    let s = state.clone();
    let url_clone = url.clone();
    tokio::spawn(async move {
        google_calendar_sync::run(&s, &url_clone, google_calendar_room_id).await;
    });

    let s = state.clone();
    let cron = google_calendar_sync_cron.clone();
    scheduler
        .add(Job::new_async_tz(&cron, Paris, move |_id, _sched| {
            let s = s.clone();
            let url = url.clone();
            Box::pin(async move { google_calendar_sync::run(&s, &url, google_calendar_room_id).await })
        })?)
        .await?;

    tracing::info!(
        google_calendar_sync_cron,
        room_id = google_calendar_room_id,
        "Google Calendar sync scheduled"
    );

    Ok(())
}
