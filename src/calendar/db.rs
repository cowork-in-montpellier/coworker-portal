use anyhow::Result;
use sqlx::SqlitePool;

/// Opens the calendar database, creating it on first use, and brings its schema up to date.
pub async fn connect(url: &str) -> Result<SqlitePool> {
    let db = crate::sqlite::open(url, "CALENDAR_DATABASE_URL").await?;
    sqlx::migrate!("./migrations_calendar").run(&db).await?;
    tracing::info!("Calendar: database ready");
    Ok(db)
}

/// An empty, migrated in-memory database, for tests.
#[cfg(test)]
pub async fn in_memory() -> SqlitePool {
    let db = crate::sqlite::open_in_memory().await;
    sqlx::migrate!("./migrations_calendar").run(&db).await.unwrap();
    db
}
