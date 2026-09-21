use anyhow::{Context, Result};
use chrono::NaiveDate;

use super::{repository, source, state::State};

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

    repository::upsert(&state.db, date, &word, &possible_words).await?;

    Ok(repository::DailyWord { word, possible_words })
}

pub fn today_paris() -> NaiveDate {
    chrono::Utc::now().with_timezone(&chrono_tz::Europe::Paris).date_naive()
}
