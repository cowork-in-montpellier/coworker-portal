use anyhow::{Context, Result};
use chrono::NaiveDate;

use super::{domain, repository, source, state::State};

/// Size of the "play a previous day" window, inclusive of today.
pub const HISTORY_WINDOW_DAYS: i64 = 30;

/// Returns today's word and dictionary, fetching and caching it from the source site
/// on first use of the day (the cron task normally warms this before anyone asks).
pub async fn ensure_daily_word(state: &State, date: NaiveDate) -> Result<repository::DailyWord> {
    if let Some(existing) = repository::get_by_date(&state.db, date).await? {
        return Ok(existing);
    }

    let word = source::fetch_word_of_day(&state.http, date).await?;
    let first_letter = word.chars().next().context("SUTOM word of the day was empty")?;
    let length = word.chars().count();
    let possible_words = source::fetch_possible_words(&state.http, length, first_letter).await?;
    let puzzle_number = domain::puzzle_number(date);

    repository::upsert_word(&state.db, date, &word, &possible_words, puzzle_number).await?;

    Ok(repository::DailyWord { word, possible_words, puzzle_number, par: None })
}

pub fn today_paris() -> NaiveDate {
    chrono::Utc::now().with_timezone(&chrono_tz::Europe::Paris).date_naive()
}

pub fn history_window_start(today: NaiveDate) -> NaiveDate {
    today - chrono::Duration::days(HISTORY_WINDOW_DAYS - 1)
}

/// Whether `date` may still be played today — not in the future, and within the
/// last `HISTORY_WINDOW_DAYS` days.
pub fn is_playable(date: NaiveDate, today: NaiveDate) -> bool {
    date <= today && date >= history_window_start(today)
}

/// Freezes yesterday's par from whoever finished it, run once per night right before
/// today's word is warmed. A no-op if it was already set or nobody finished that day.
pub async fn finalize_par_for(state: &State, date: NaiveDate) -> Result<()> {
    if let Some(par) = repository::average_score_ceil(&state.db, date).await? {
        repository::set_par_if_missing(&state.db, date, par).await?;
    }
    Ok(())
}

/// The par to use right now: the frozen value once the nightly job has set it, otherwise
/// a live, still-moving estimate from whoever has finished the day so far. `None` only
/// when nobody has finished the day at all yet.
pub async fn effective_par(state: &State, date: NaiveDate, frozen: Option<i32>) -> Result<Option<i32>, sqlx::Error> {
    match frozen {
        Some(par) => Ok(Some(par)),
        None => repository::average_score_ceil(&state.db, date).await,
    }
}
