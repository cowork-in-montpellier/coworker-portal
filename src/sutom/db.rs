use std::str::FromStr;

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

/// Opens the SUTOM database, creating the file (and its directory) on first use, and
/// brings its schema up to date.
pub async fn connect(url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(url)
        .with_context(|| format!("Invalid SUTOM_DATABASE_URL {url:?}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    if let Some(dir) = options.get_filename().parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("Creating SUTOM database directory {dir:?}"))?;
    }

    let db = SqlitePoolOptions::new().max_connections(5).connect_with(options).await?;
    sqlx::migrate!("./migrations_sutom").run(&db).await?;
    tracing::info!("Sutom: database ready");
    Ok(db)
}

/// An empty, migrated in-memory database, for tests.
#[cfg(test)]
pub async fn in_memory() -> SqlitePool {
    // A single connection: each new in-memory connection would be a brand new database.
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true))
        .await
        .unwrap();
    sqlx::migrate!("./migrations_sutom").run(&db).await.unwrap();
    db
}
