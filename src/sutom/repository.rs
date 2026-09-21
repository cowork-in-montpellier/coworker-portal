use chrono::NaiveDate;
use sqlx::PgPool;

pub struct DailyWord {
    pub word: String,
    pub possible_words: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct DailyWordRow {
    word: String,
    possible_words: Vec<String>,
}

pub async fn get_by_date(db: &PgPool, date: NaiveDate) -> Result<Option<DailyWord>, sqlx::Error> {
    let row = sqlx::query_as::<_, DailyWordRow>(
        "SELECT word, possible_words FROM portal_sutom_word WHERE game_date = $1",
    )
    .bind(date)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| DailyWord { word: r.word, possible_words: r.possible_words }))
}

pub async fn upsert(
    db: &PgPool,
    date: NaiveDate,
    word: &str,
    possible_words: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO portal_sutom_word (game_date, word, possible_words)
         VALUES ($1, $2, $3)
         ON CONFLICT (game_date) DO UPDATE SET word = EXCLUDED.word, possible_words = EXCLUDED.possible_words",
    )
    .bind(date)
    .bind(word)
    .bind(possible_words)
    .execute(db)
    .await?;

    Ok(())
}
