mod refresh;

use chrono_tz::Europe::Paris;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::sutom::state::State;

/// Register the sutom module's background tasks on the shared scheduler.
pub async fn register(scheduler: &JobScheduler, state: State) -> anyhow::Result<()> {
    let refresh_cron = state.config.refresh_cron.clone();
    refresh_cron.parse::<croner::Cron>()
        .map_err(|e| anyhow::anyhow!("Invalid SUTOM_REFRESH_CRON {:?}: {}", refresh_cron, e))?;

    // Word refresh: shortly after midnight (Paris) every day.
    let s = state.clone();
    let cron = refresh_cron.clone();
    scheduler.add(Job::new_async_tz(&cron, Paris, move |_id, _sched| {
        let s = s.clone();
        Box::pin(async move { refresh::run(&s).await })
    })?).await?;

    tracing::info!(refresh_cron, timezone = "Europe/Paris", "Sutom scheduler tasks registered");

    Ok(())
}
