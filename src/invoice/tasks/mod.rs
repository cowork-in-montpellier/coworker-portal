mod monthly_usage;
mod voucher_sync;

use chrono_tz::Europe::Paris;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::invoice::state::State;

/// Register the invoice module's background tasks on the shared scheduler.
pub async fn register(scheduler: &JobScheduler, state: State) -> anyhow::Result<()> {
    let voucher_sync_cron = state.config.voucher_sync_cron.clone();
    let monthly_usage_cron = state.config.monthly_usage_cron.clone();

    voucher_sync_cron.parse::<croner::Cron>()
        .map_err(|e| anyhow::anyhow!("Invalid VOUCHER_SYNC_CRON {:?}: {}", voucher_sync_cron, e))?;
    monthly_usage_cron.parse::<croner::Cron>()
        .map_err(|e| anyhow::anyhow!("Invalid MONTHLY_USAGE_CRON {:?}: {}", monthly_usage_cron, e))?;

    // Voucher sync: Mon–Fri, every hour 09:00–19:00 (Paris).
    let s = state.clone();
    let cron = voucher_sync_cron.clone();
    scheduler.add(Job::new_async_tz(&cron, Paris, move |_id, _sched| {
        let s = s.clone();
        Box::pin(async move { voucher_sync::run(&s).await })
    })?).await?;

    // Monthly usage diary: every day at 23:00 (Paris).
    let s = state.clone();
    let cron = monthly_usage_cron.clone();
    scheduler.add(Job::new_async_tz(&cron, Paris, move |_id, _sched| {
        let s = s.clone();
        Box::pin(async move { monthly_usage::run(&s).await })
    })?).await?;

    tracing::info!(
        voucher_sync_cron,
        monthly_usage_cron,
        timezone = "Europe/Paris",
        "Invoice scheduler tasks registered"
    );

    Ok(())
}
