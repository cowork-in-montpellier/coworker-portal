use chrono::NaiveDate;
use sqlx::PgPool;

// ---- Daily word ----

pub struct DailyWord {
    pub word: String,
    pub possible_words: Vec<String>,
    pub puzzle_number: i32,
    pub par: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct DailyWordRow {
    word: String,
    possible_words: Vec<String>,
    puzzle_number: i32,
    par: Option<i32>,
}

pub async fn get_by_date(db: &PgPool, date: NaiveDate) -> Result<Option<DailyWord>, sqlx::Error> {
    let row = sqlx::query_as::<_, DailyWordRow>(
        "SELECT word, possible_words, puzzle_number, par FROM portal_sutom_word WHERE game_date = $1",
    )
    .bind(date)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| DailyWord {
        word: r.word,
        possible_words: r.possible_words,
        puzzle_number: r.puzzle_number,
        par: r.par,
    }))
}

pub async fn upsert_word(
    db: &PgPool,
    date: NaiveDate,
    word: &str,
    possible_words: &[String],
    puzzle_number: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO portal_sutom_word (game_date, word, possible_words, puzzle_number)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (game_date) DO UPDATE SET word = EXCLUDED.word, possible_words = EXCLUDED.possible_words",
    )
    .bind(date)
    .bind(word)
    .bind(possible_words)
    .bind(puzzle_number)
    .execute(db)
    .await?;

    Ok(())
}

/// Freezes the par for a day, but only if it hasn't been set yet — pars never change
/// once computed, even if more players catch up on that day afterwards.
pub async fn set_par_if_missing(db: &PgPool, date: NaiveDate, par: i32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE portal_sutom_word SET par = $1 WHERE game_date = $2 AND par IS NULL")
        .bind(par)
        .bind(date)
        .execute(db)
        .await?;
    Ok(())
}

/// Computes ceil(avg(score)) among finished attempts for a date, or `None` if nobody
/// finished that day.
pub async fn average_score_ceil(db: &PgPool, date: NaiveDate) -> Result<Option<i32>, sqlx::Error> {
    let avg: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(score)::float8 FROM portal_sutom_attempt WHERE game_date = $1 AND score IS NOT NULL",
    )
    .bind(date)
    .fetch_one(db)
    .await?;

    Ok(avg.map(|a| a.ceil() as i32))
}

pub async fn list_recent_dates(
    db: &PgPool,
    since: NaiveDate,
) -> Result<Vec<(NaiveDate, i32)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT game_date, puzzle_number FROM portal_sutom_word \
         WHERE game_date >= $1 ORDER BY game_date DESC",
    )
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
    guesses: Vec<String>,
    score: Option<i32>,
}

pub async fn get_attempt(
    db: &PgPool,
    user_id: i32,
    date: NaiveDate,
) -> Result<Option<Attempt>, sqlx::Error> {
    let row = sqlx::query_as::<_, AttemptRow>(
        "SELECT guesses, score FROM portal_sutom_attempt WHERE user_id = $1 AND game_date = $2",
    )
    .bind(user_id)
    .bind(date)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| Attempt { guesses: r.guesses, score: r.score }))
}

pub async fn upsert_attempt(
    db: &PgPool,
    user_id: i32,
    date: NaiveDate,
    guesses: &[String],
    score: Option<i32>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO portal_sutom_attempt (user_id, game_date, guesses, score, updated_at)
         VALUES ($1, $2, $3, $4, now())
         ON CONFLICT (user_id, game_date)
         DO UPDATE SET guesses = EXCLUDED.guesses, score = EXCLUDED.score, updated_at = now()",
    )
    .bind(user_id)
    .bind(date)
    .bind(guesses)
    .bind(score)
    .execute(db)
    .await?;
    Ok(())
}

pub struct PlayerAttempt {
    pub user_id: i32,
    pub first_name: String,
    pub guesses: Vec<String>,
    pub score: Option<i32>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// All players who have started today's (or a given day's) puzzle, finished ones first
/// (best score first), unfinished ones last.
pub async fn list_attempts_for_date(
    db: &PgPool,
    date: NaiveDate,
) -> Result<Vec<PlayerAttempt>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        user_id: i32,
        first_name: String,
        guesses: Vec<String>,
        score: Option<i32>,
        updated_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT a.user_id, u.first_name, a.guesses, a.score, a.updated_at
         FROM portal_sutom_attempt a
         JOIN auth_user u ON u.id = a.user_id
         WHERE a.game_date = $1
         ORDER BY (a.score IS NULL), a.score, u.first_name",
    )
    .bind(date)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PlayerAttempt {
            user_id: r.user_id,
            first_name: r.first_name,
            guesses: r.guesses,
            score: r.score,
            updated_at: r.updated_at,
        })
        .collect())
}

pub struct ScoreRow {
    pub user_id: i32,
    pub first_name: String,
    pub score: i32,
    pub par: i32,
    pub game_date: NaiveDate,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_first: bool,
}

/// Every finished (scored) attempt in the last 30 days whose day already has a frozen
/// par, joined with the player's name — the raw material for the leaderboard. `is_first`
/// flags whichever attempt was earliest to finish that particular day.
pub async fn list_scored_attempts_since(
    db: &PgPool,
    since: NaiveDate,
) -> Result<Vec<ScoreRow>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        user_id: i32,
        first_name: String,
        score: i32,
        par: i32,
        game_date: NaiveDate,
        updated_at: chrono::DateTime<chrono::Utc>,
        is_first: bool,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT a.user_id, u.first_name, a.score, w.par, a.game_date, a.updated_at,
                (RANK() OVER (PARTITION BY a.game_date ORDER BY a.updated_at) = 1) AS is_first
         FROM portal_sutom_attempt a
         JOIN auth_user u ON u.id = a.user_id
         JOIN portal_sutom_word w ON w.game_date = a.game_date
         WHERE a.game_date >= $1 AND a.score IS NOT NULL AND w.par IS NOT NULL",
    )
    .bind(since)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ScoreRow {
            user_id: r.user_id,
            first_name: r.first_name,
            score: r.score,
            par: r.par,
            game_date: r.game_date,
            updated_at: r.updated_at,
            is_first: r.is_first,
        })
        .collect())
}
