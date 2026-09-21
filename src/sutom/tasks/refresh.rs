use crate::sutom::{service, state::State};

pub async fn run(state: &State) {
    let today = service::today_paris();
    tracing::info!(date = %today, "Sutom refresh: starting");

    match service::ensure_daily_word(state, today).await {
        Ok(daily) => {
            tracing::info!(date = %today, length = daily.word.chars().count(), "Sutom refresh: done")
        }
        Err(e) => tracing::error!(date = %today, error = %e, "Sutom refresh: failed"),
    }
}
