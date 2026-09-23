use crate::sutom::{service, state::State};

pub async fn run(state: &State) {
    let today = service::today_paris();
    let yesterday = today - chrono::Duration::days(1);

    tracing::info!(date = %yesterday, "Sutom par: finalizing yesterday");
    if let Err(e) = service::finalize_par_for(state, yesterday).await {
        tracing::error!(date = %yesterday, error = %e, "Sutom par: failed");
    }

    tracing::info!(date = %today, "Sutom refresh: starting");
    match service::ensure_daily_word(state, today).await {
        Ok(daily) => {
            tracing::info!(date = %today, length = daily.word.chars().count(), "Sutom refresh: done");
            if let Err(e) = service::possible_words(state, &daily.word).await {
                tracing::error!(date = %today, error = %e, "Sutom refresh: dictionary warm-up failed");
            }
        }
        Err(e) => tracing::error!(date = %today, error = %e, "Sutom refresh: failed"),
    }
}
