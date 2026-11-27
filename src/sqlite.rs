//! Opening the modules' own SQLite databases (SUTOM, calendar), each in its own file and
//! kept apart from the shared PostgreSQL one. Each module runs its own migrations.

use std::str::FromStr;

use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

/// Opens a SQLite database from its URL, creating the file (and its directory) on first use.
/// `env_var` only names where the URL came from, for error messages.
pub async fn open(url: &str, env_var: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(url)
        .with_context(|| format!("Invalid {env_var} {url:?}"))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

    if let Some(dir) = options.get_filename().parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("Creating database directory {dir:?} for {env_var}"))?;
    }

    Ok(SqlitePoolOptions::new().max_connections(5).connect_with(options).await?)
}

/// An empty in-memory database, for tests.
#[cfg(test)]
pub async fn open_in_memory() -> SqlitePool {
    // A single connection: each new in-memory connection would be a brand new database.
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true))
        .await
        .unwrap()
}
