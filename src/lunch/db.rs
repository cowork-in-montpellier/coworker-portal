use anyhow::Result;
use sqlx::SqlitePool;

/// Opens the Pause-déj database, creating it on first use, and brings its schema up to date.
pub async fn connect(url: &str) -> Result<SqlitePool> {
    let db = crate::sqlite::open(url, "LUNCH_DATABASE_URL").await?;
    sqlx::migrate!("./migrations_lunch").run(&db).await?;
    tracing::info!("Lunch: database ready");
    Ok(db)
}

/// An empty, migrated in-memory database (seeded places included), for tests.
#[cfg(test)]
pub async fn in_memory() -> SqlitePool {
    let db = crate::sqlite::open_in_memory().await;
    sqlx::migrate!("./migrations_lunch").run(&db).await.unwrap();
    db
}
