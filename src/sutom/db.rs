use anyhow::Result;
use sqlx::SqlitePool;

/// Opens the SUTOM database, creating it on first use, and brings its schema up to date.
pub async fn connect(url: &str) -> Result<SqlitePool> {
    let db = crate::sqlite::open(url, "SUTOM_DATABASE_URL").await?;
    sqlx::migrate!("./migrations_sutom").run(&db).await?;
    tracing::info!("Sutom: database ready");
    Ok(db)
}

/// An empty, migrated in-memory database, for tests.
#[cfg(test)]
pub async fn in_memory() -> SqlitePool {
    let db = crate::sqlite::open_in_memory().await;
    sqlx::migrate!("./migrations_sutom").run(&db).await.unwrap();
    db
}
