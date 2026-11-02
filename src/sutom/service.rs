use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{Months, NaiveDate};

use super::{domain, repository, source, state::State};

/// Size of the "play a previous day" window, inclusive of today.
pub const HISTORY_WINDOW_DAYS: i64 = 30;

/// Returns a day's word, fetching and storing it from the source site on first use of
/// the day (the cron task normally warms this before anyone asks).
pub async fn ensure_daily_word(state: &State, date: NaiveDate) -> Result<repository::DailyWord> {
    if let Some(existing) = repository::get_by_date(&state.db, date).await? {
        return Ok(existing);
    }

    let word = source::fetch_word_of_day(&state.http, date).await?;
    let puzzle_number = domain::puzzle_number(date);

    repository::upsert_word(&state.db, date, &word, puzzle_number).await?;

    Ok(repository::DailyWord { word, puzzle_number, par: None })
}

/// The guessable-words dictionary for a target word: every word with its length and
/// first letter, served from the in-memory cache and fetched from the source site on a miss.
pub async fn possible_words(state: &State, word: &str) -> Result<Arc<Vec<String>>> {
    let first_letter = word.chars().next().context("SUTOM word of the day was empty")?;
    let length = word.chars().count();
    state.dictionaries.get(&state.http, length, first_letter).await
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

/// One player's totals over a set of finished attempts.
pub struct Totals {
    pub user_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub points: f64,
    pub games_played: i32,
    score_sum: i32,
}

impl Totals {
    /// Average number of guesses over the games played (a failure counts as 7).
    pub fn avg_guesses(&self) -> f64 {
        self.score_sum as f64 / self.games_played as f64
    }
}

/// Sums each player's points over `rows`, best first: most points, then the lowest
/// average number of guesses, then the most games played. A perfect tie on all three
/// falls back to the user id, so the order (and a month's winner) is always the same.
pub fn rank(rows: Vec<repository::ScoreRow>) -> Vec<Totals> {
    let mut totals: HashMap<i32, Totals> = HashMap::new();
    for row in rows {
        let days_late = domain::days_late(row.game_date, row.updated_at);
        let fast = domain::is_fast_finish(row.started_at, row.updated_at, row.score);
        let points = domain::leaderboard_points(row.par, row.score, row.is_first, fast, days_late);
        let entry = totals.entry(row.user_id).or_insert_with(|| Totals {
            user_id: row.user_id,
            first_name: row.first_name,
            last_name: row.last_name,
            points: 0.0,
            games_played: 0,
            score_sum: 0,
        });
        entry.points += points;
        entry.games_played += 1;
        entry.score_sum += row.score;
    }

    let mut ranked: Vec<Totals> = totals.into_values().collect();
    ranked.sort_by(|a, b| {
        b.points
            .partial_cmp(&a.points)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.avg_guesses().partial_cmp(&b.avg_guesses()).unwrap_or(Ordering::Equal))
            .then_with(|| b.games_played.cmp(&a.games_played))
            .then_with(|| a.user_id.cmp(&b.user_id))
    });
    ranked
}

/// A month's ranking computed from the attempts: games on that month's grids finished
/// before the month ended (and before `today`, for the current month), best first.
pub async fn compute_month_ranking(
    state: &State,
    month: NaiveDate,
    today: NaiveDate,
) -> Result<Vec<repository::MonthRankingEntry>, sqlx::Error> {
    let (first, last) = domain::month_bounds(month);
    // Days before the competition's official start were warm-up/training and never count.
    let from = first.max(domain::leaderboard_start_date());
    let to = last.min(today);
    if from > to {
        return Ok(Vec::new());
    }

    let rows = repository::list_scored_attempts_between(&state.db, from, to, domain::month_end(first)).await?;

    Ok(rank(rows)
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            repository::MonthRankingEntry {
                rank: i as i32 + 1,
                avg_guesses: t.avg_guesses(),
                user_id: t.user_id,
                first_name: t.first_name,
                last_name: t.last_name,
                points: t.points,
                games_played: t.games_played,
            }
        })
        .collect())
}

/// Freezes the ranking of every completed month since `first_titled_month` that isn't
/// frozen yet; its winner earns a star. Run nightly and at startup. A month nobody played has nothing to freeze and is retried next time.
pub async fn finalize_month_rankings(state: &State, today: NaiveDate) -> Result<()> {
    let (current_month, _) = domain::month_bounds(today);
    let frozen: HashSet<NaiveDate> = repository::frozen_months(&state.db).await?.into_iter().collect();

    let mut month = domain::first_titled_month();
    while month < current_month {
        if !frozen.contains(&month) {
            let entries = compute_month_ranking(state, month, today).await?;
            if !entries.is_empty() {
                repository::insert_month_ranking(&state.db, month, &entries).await?;
                tracing::info!(%month, players = entries.len(), winner = entries[0].user_id, "Sutom: month ranking frozen");
            }
        }
        month = month.checked_add_months(Months::new(1)).context("month out of range")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(user_id: i32, score: i32, par: i32, day: u32) -> repository::ScoreRow {
        let game_date = NaiveDate::from_ymd_opt(2026, 10, day).unwrap();
        repository::ScoreRow {
            user_id,
            first_name: format!("P{user_id}"),
            last_name: String::new(),
            score,
            par,
            game_date,
            started_at: None,
            updated_at: game_date.and_hms_opt(10, 0, 0).unwrap().and_utc(),
            is_first: false,
        }
    }

    fn order(rows: Vec<repository::ScoreRow>) -> Vec<i32> {
        rank(rows).into_iter().map(|t| t.user_id).collect()
    }

    #[test]
    fn rank_breaks_equal_points_by_lower_average() {
        // Both total 6 points: player 1 with two par games (avg 4), player 2 with a single
        // first-guess win on par 4 (avg 1), so player 2 ranks first.
        let rows = vec![row(1, 4, 4, 1), row(1, 4, 4, 2), row(2, 1, 4, 1)];
        let ranked = rank(rows);
        assert_eq!(ranked.iter().map(|t| t.points).collect::<Vec<_>>(), vec![6.0, 6.0]);
        assert_eq!(ranked.iter().map(|t| t.user_id).collect::<Vec<_>>(), vec![2, 1]);
    }

    #[test]
    fn rank_is_stable_on_a_perfect_tie() {
        for _ in 0..20 {
            assert_eq!(order(vec![row(7, 4, 4, 1), row(3, 4, 4, 1), row(5, 4, 4, 1)]), vec![3, 5, 7]);
        }
    }
}
