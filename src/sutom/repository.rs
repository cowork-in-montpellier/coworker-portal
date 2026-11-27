use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;
use sqlx::types::Json;

// ---- Daily word ----

pub struct DailyWord {
    pub word: String,
    pub puzzle_number: i32,
    pub par: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct DailyWordRow {
    word: String,
    puzzle_number: i32,
    par: Option<i32>,
}

pub async fn get_by_date(db: &SqlitePool, date: NaiveDate) -> Result<Option<DailyWord>, sqlx::Error> {
    let row = sqlx::query_as::<_, DailyWordRow>("SELECT word, puzzle_number, par FROM word WHERE game_date = $1")
        .bind(date)
        .fetch_optional(db)
        .await?;

    Ok(row.map(|r| DailyWord {
        word: r.word,
        puzzle_number: r.puzzle_number,
        par: r.par,
    }))
}

pub async fn upsert_word(
    db: &SqlitePool,
    date: NaiveDate,
    word: &str,
    puzzle_number: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO word (game_date, word, puzzle_number)
         VALUES ($1, $2, $3)
         ON CONFLICT (game_date) DO UPDATE SET word = excluded.word",
    )
    .bind(date)
    .bind(word)
    .bind(puzzle_number)
    .execute(db)
    .await?;

    Ok(())
}

/// Freezes the par for a day, but only if it hasn't been set yet — pars never change
/// once computed, even if more players catch up on that day afterwards.
pub async fn set_par_if_missing(db: &SqlitePool, date: NaiveDate, par: i32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE word SET par = $1 WHERE game_date = $2 AND par IS NULL")
        .bind(par)
        .bind(date)
        .execute(db)
        .await?;
    Ok(())
}

/// Raw (unrounded) average score among finished attempts for a date, or `None` if
/// nobody finished that day.
pub async fn average_score(db: &SqlitePool, date: NaiveDate) -> Result<Option<f64>, sqlx::Error> {
    sqlx::query_scalar("SELECT AVG(score) FROM attempt WHERE game_date = $1 AND score IS NOT NULL")
        .bind(date)
        .fetch_one(db)
        .await
}

/// Computes ceil(avg(score)) among finished attempts for a date, or `None` if nobody
/// finished that day.
pub async fn average_score_ceil(db: &SqlitePool, date: NaiveDate) -> Result<Option<i32>, sqlx::Error> {
    Ok(average_score(db, date).await?.map(|a| a.ceil() as i32))
}

pub async fn list_recent_dates(db: &SqlitePool, since: NaiveDate) -> Result<Vec<(NaiveDate, i32)>, sqlx::Error> {
    sqlx::query_as("SELECT game_date, puzzle_number FROM word WHERE game_date >= $1 ORDER BY game_date DESC")
        .bind(since)
        .fetch_all(db)
        .await
}

// ---- Attempts ----

pub struct Attempt {
    pub guesses: Vec<String>,
    pub score: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct AttemptRow {
    guesses: Json<Vec<String>>,
    score: Option<i32>,
}

pub async fn get_attempt(db: &SqlitePool, user_id: i32, date: NaiveDate) -> Result<Option<Attempt>, sqlx::Error> {
    let row = sqlx::query_as::<_, AttemptRow>("SELECT guesses, score FROM attempt WHERE user_id = $1 AND game_date = $2")
        .bind(user_id)
        .bind(date)
        .fetch_optional(db)
        .await?;

    Ok(row.map(|r| Attempt { guesses: r.guesses.0, score: r.score }))
}

pub async fn upsert_attempt(
    db: &SqlitePool,
    user_id: i32,
    date: NaiveDate,
    guesses: &[String],
    score: Option<i32>,
    possible_counts: Option<&[i32]>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO attempt (user_id, game_date, guesses, score, possible_counts, started_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $6)
         ON CONFLICT (user_id, game_date)
         DO UPDATE SET guesses = excluded.guesses, score = excluded.score,
                       possible_counts = excluded.possible_counts, updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind(date)
    .bind(Json(guesses))
    .bind(score)
    .bind(possible_counts.map(Json))
    .bind(Utc::now())
    .execute(db)
    .await?;
    Ok(())
}

/// Stores the possible-words counts of a game finished before they were recorded.
pub async fn set_possible_counts(
    db: &SqlitePool,
    user_id: i32,
    date: NaiveDate,
    possible_counts: &[i32],
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE attempt SET possible_counts = $3 WHERE user_id = $1 AND game_date = $2")
        .bind(user_id)
        .bind(date)
        .bind(Json(possible_counts))
        .execute(db)
        .await?;
    Ok(())
}

pub struct PlayerAttempt {
    pub user_id: i32,
    pub guesses: Vec<String>,
    pub score: Option<i32>,
    /// Possible words when each guess was made, once the game is finished.
    pub possible_counts: Option<Vec<i32>>,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// All players who have started a given day's puzzle.
pub async fn list_attempts_for_date(db: &SqlitePool, date: NaiveDate) -> Result<Vec<PlayerAttempt>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        user_id: i32,
        guesses: Json<Vec<String>>,
        score: Option<i32>,
        possible_counts: Option<Json<Vec<i32>>>,
        started_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT user_id, guesses, score, possible_counts, started_at, updated_at FROM attempt WHERE game_date = $1",
    )
    .bind(date)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PlayerAttempt {
            user_id: r.user_id,
            guesses: r.guesses.0,
            score: r.score,
            possible_counts: r.possible_counts.map(|c| c.0),
            started_at: r.started_at,
            updated_at: r.updated_at,
        })
        .collect())
}

pub struct ScoreRow {
    pub user_id: i32,
    pub score: i32,
    /// The frozen par, or for a day not frozen yet (e.g. today) the live estimate from
    /// the attempts finished so far.
    pub par: i32,
    pub game_date: NaiveDate,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub is_first: bool,
}

/// Every attempt with a game date in `[from, to]` finished before `finished_before` —
/// the raw material for the leaderboard. `is_first` flags whichever of those attempts was
/// earliest to finish that particular day.
pub async fn list_scored_attempts_between(
    db: &SqlitePool,
    from: NaiveDate,
    to: NaiveDate,
    finished_before: DateTime<Utc>,
) -> Result<Vec<ScoreRow>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        user_id: i32,
        score: i32,
        game_date: NaiveDate,
        started_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
        frozen_par: Option<i32>,
        average_score: f64,
        is_first: bool,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT a.user_id, a.score, a.game_date, a.started_at, a.updated_at,
                w.par AS frozen_par,
                AVG(a.score) OVER (PARTITION BY a.game_date) AS average_score,
                (RANK() OVER (PARTITION BY a.game_date ORDER BY a.updated_at) = 1) AS is_first
         FROM attempt a
         JOIN word w ON w.game_date = a.game_date
         WHERE a.game_date BETWEEN $1 AND $2 AND a.score IS NOT NULL AND a.updated_at < $3",
    )
    .bind(from)
    .bind(to)
    .bind(finished_before)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ScoreRow {
            user_id: r.user_id,
            score: r.score,
            par: r.frozen_par.unwrap_or(r.average_score.ceil() as i32),
            game_date: r.game_date,
            started_at: r.started_at,
            updated_at: r.updated_at,
            is_first: r.is_first,
        })
        .collect())
}

// ---- Monthly rankings ----

/// One player's line in a month's ranking, frozen or computed live.
#[derive(sqlx::FromRow)]
pub struct MonthRankingEntry {
    pub user_id: i32,
    /// 1 = winner of the month.
    pub rank: i32,
    pub points: f64,
    pub games_played: i32,
    pub avg_guesses: f64,
}

/// A month's frozen ranking, best first, or empty if that month isn't frozen.
pub async fn month_ranking(db: &SqlitePool, month: NaiveDate) -> Result<Vec<MonthRankingEntry>, sqlx::Error> {
    sqlx::query_as(
        "SELECT user_id, rank, points, games_played, avg_guesses FROM month_ranking WHERE month = $1 ORDER BY rank",
    )
    .bind(month)
    .fetch_all(db)
    .await
}

/// First day of every month whose ranking is frozen.
pub async fn frozen_months(db: &SqlitePool) -> Result<Vec<NaiveDate>, sqlx::Error> {
    sqlx::query_scalar("SELECT DISTINCT month FROM month_ranking").fetch_all(db).await
}

/// Each player's stars: how many frozen months they won, as (user id, count).
pub async fn star_counts(db: &SqlitePool) -> Result<Vec<(i32, i64)>, sqlx::Error> {
    sqlx::query_as("SELECT user_id, COUNT(*) FROM month_ranking WHERE rank = 1 GROUP BY user_id")
        .fetch_all(db)
        .await
}

/// Freezes a month's whole ranking in one transaction, so a month is either fully
/// frozen or not at all. Rows already there are left untouched.
pub async fn insert_month_ranking(
    db: &SqlitePool,
    month: NaiveDate,
    entries: &[MonthRankingEntry],
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    for e in entries {
        sqlx::query(
            "INSERT INTO month_ranking (month, user_id, rank, points, games_played, avg_guesses)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (month, user_id) DO NOTHING",
        )
        .bind(month)
        .bind(e.user_id)
        .bind(e.rank)
        .bind(e.points)
        .bind(e.games_played)
        .bind(e.avg_guesses)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sutom::db;
    use chrono::Datelike;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn guesses(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[tokio::test]
    async fn attempts_round_trip_guesses_and_counts() {
        let db = db::in_memory().await;
        upsert_word(&db, date(1), "BILLE", 1).await.unwrap();
        upsert_attempt(&db, 7, date(1), &guesses(&["BELLE"]), None, None).await.unwrap();
        upsert_attempt(&db, 7, date(1), &guesses(&["BELLE", "BILLE"]), Some(2), Some(&[5, 3])).await.unwrap();

        let attempt = get_attempt(&db, 7, date(1)).await.unwrap().unwrap();
        assert_eq!(attempt.guesses, guesses(&["BELLE", "BILLE"]));
        assert_eq!(attempt.score, Some(2));

        let listed = list_attempts_for_date(&db, date(1)).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].possible_counts, Some(vec![5, 3]));
        // The first guess's time is kept when the game goes on.
        assert!(listed[0].started_at.unwrap() <= listed[0].updated_at);
    }

    #[tokio::test]
    async fn scored_attempts_use_the_live_par_until_frozen_and_flag_the_first() {
        let db = db::in_memory().await;
        upsert_word(&db, date(1), "BILLE", 1).await.unwrap();
        upsert_word(&db, date(2), "BALLE", 2).await.unwrap();
        upsert_attempt(&db, 1, date(1), &guesses(&["BILLE"]), Some(3), None).await.unwrap();
        upsert_attempt(&db, 2, date(1), &guesses(&["BILLE"]), Some(4), None).await.unwrap();
        upsert_attempt(&db, 1, date(2), &guesses(&["BALLE"]), Some(2), None).await.unwrap();
        // Still in progress: never counted.
        upsert_attempt(&db, 2, date(2), &guesses(&["BELLE"]), None, None).await.unwrap();
        set_par_if_missing(&db, date(2), 5).await.unwrap();

        let mut rows = list_scored_attempts_between(&db, date(1), date(2), Utc::now() + chrono::Duration::hours(1))
            .await
            .unwrap();
        rows.sort_by_key(|r| (r.game_date, r.user_id));
        let summary: Vec<_> = rows.iter().map(|r| (r.game_date.day0() + 1, r.user_id, r.par, r.is_first)).collect();
        // Day 1: live par ceil(3.5) = 4, player 1 finished first. Day 2: frozen par 5.
        assert_eq!(summary, vec![(1, 1, 4, true), (1, 2, 4, false), (2, 1, 5, true)]);

        assert!(list_scored_attempts_between(&db, date(1), date(2), Utc::now() - chrono::Duration::hours(1))
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn month_rankings_count_stars() {
        let db = db::in_memory().await;
        let entry = |user_id, rank| MonthRankingEntry { user_id, rank, points: 10.0, games_played: 3, avg_guesses: 4.0 };
        insert_month_ranking(&db, date(1), &[entry(1, 1), entry(2, 2)]).await.unwrap();
        let november = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        insert_month_ranking(&db, november, &[entry(1, 1)]).await.unwrap();

        assert_eq!(month_ranking(&db, date(1)).await.unwrap().iter().map(|e| e.user_id).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(frozen_months(&db).await.unwrap().len(), 2);
        assert_eq!(star_counts(&db).await.unwrap(), vec![(1, 2)]);
    }
}
