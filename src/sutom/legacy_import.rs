//! One-time copy of the SUTOM data from the main PostgreSQL database, where it lived before
//! moving to its own SQLite database. Runs at startup and only does anything while the
//! SQLite database is still empty and the old `portal_sutom_*` tables exist.
//!
//! The old tables are left untouched. Once every environment has been migrated, they can
//! be dropped by a PostgreSQL migration and this module removed.

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::types::Json;
use sqlx::{PgPool, SqlitePool};

type WordRow = (NaiveDate, String, i32, Option<i32>);
type AttemptRow = (i32, NaiveDate, Vec<String>, Option<i32>, Option<Vec<i32>>, Option<DateTime<Utc>>, DateTime<Utc>);
type RankingRow = (NaiveDate, i32, i32, f64, i32, f64);

pub async fn run(pg: &PgPool, sqlite: &SqlitePool) -> Result<()> {
    let already_filled: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM word)").fetch_one(sqlite).await?;
    if already_filled {
        return Ok(());
    }
    let has_legacy: bool = sqlx::query_scalar("SELECT to_regclass('portal_sutom_word') IS NOT NULL")
        .fetch_one(pg)
        .await?;
    if !has_legacy {
        return Ok(());
    }

    let words: Vec<WordRow> =
        sqlx::query_as("SELECT game_date, word, puzzle_number, par FROM portal_sutom_word").fetch_all(pg).await?;
    let attempts: Vec<AttemptRow> = sqlx::query_as(
        "SELECT user_id, game_date, guesses, score, possible_counts, started_at, updated_at FROM portal_sutom_attempt",
    )
    .fetch_all(pg)
    .await?;
    let rankings: Vec<RankingRow> = sqlx::query_as(
        "SELECT month, user_id, rank, points, games_played, avg_guesses FROM portal_sutom_month_ranking",
    )
    .fetch_all(pg)
    .await?;

    // All or nothing: a failed copy leaves the database empty, to be retried next start.
    let mut tx = sqlite.begin().await?;
    for (game_date, word, puzzle_number, par) in &words {
        sqlx::query("INSERT INTO word (game_date, word, puzzle_number, par) VALUES ($1, $2, $3, $4)")
            .bind(game_date)
            .bind(word)
            .bind(puzzle_number)
            .bind(par)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("copying the word of {game_date}"))?;
    }
    for (user_id, game_date, guesses, score, possible_counts, started_at, updated_at) in &attempts {
        sqlx::query(
            "INSERT INTO attempt (user_id, game_date, guesses, score, possible_counts, started_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(user_id)
        .bind(game_date)
        .bind(Json(guesses))
        .bind(score)
        .bind(possible_counts.as_ref().map(Json))
        .bind(started_at)
        .bind(updated_at)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("copying the {game_date} game of user {user_id}"))?;
    }
    for (month, user_id, rank, points, games_played, avg_guesses) in &rankings {
        sqlx::query(
            "INSERT INTO month_ranking (month, user_id, rank, points, games_played, avg_guesses)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(month)
        .bind(user_id)
        .bind(rank)
        .bind(points)
        .bind(games_played)
        .bind(avg_guesses)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("copying the {month} ranking of user {user_id}"))?;
    }
    tx.commit().await?;

    tracing::info!(
        words = words.len(),
        attempts = attempts.len(),
        rankings = rankings.len(),
        "Sutom: data copied from PostgreSQL"
    );
    Ok(())
}
