//! One-time copy of the rooms and bookings from the main PostgreSQL database, where they
//! lived before moving to the calendar's own SQLite database. Runs at startup and only does
//! anything while the SQLite database has no rooms yet and the old `portal_room*` tables exist.
//!
//! Ids are kept as they are: a booking's id is its UID in the published iCalendar feeds, and
//! `GOOGLE_CALENDAR_ROOM_ID` points at a room id.
//!
//! The old tables are left untouched. Once every environment has been migrated, they can
//! be dropped by a PostgreSQL migration and this module removed.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, SqlitePool};

type RoomRow = (i32, String, String);
type BookingRow = (i32, i32, String, DateTime<Utc>, DateTime<Utc>, Option<i32>, String, DateTime<Utc>, Option<String>);

pub async fn run(pg: &PgPool, sqlite: &SqlitePool) -> Result<()> {
    let already_filled: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM room)").fetch_one(sqlite).await?;
    if already_filled {
        return Ok(());
    }
    let has_legacy: bool =
        sqlx::query_scalar("SELECT to_regclass('portal_room') IS NOT NULL").fetch_one(pg).await?;
    if !has_legacy {
        return Ok(());
    }

    let rooms: Vec<RoomRow> = sqlx::query_as("SELECT id, name, color FROM portal_room").fetch_all(pg).await?;
    let bookings: Vec<BookingRow> = sqlx::query_as(
        "SELECT id, room_id, title, start_at, end_at, created_by, notes, created_at, google_uid
         FROM portal_room_booking",
    )
    .fetch_all(pg)
    .await?;

    // All or nothing: a failed copy leaves the database empty, to be retried next start.
    let mut tx = sqlite.begin().await?;
    for (id, name, color) in &rooms {
        sqlx::query("INSERT INTO room (id, name, color) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(name)
            .bind(color)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("copying room {id}"))?;
    }
    for (id, room_id, title, start_at, end_at, created_by, notes, created_at, google_uid) in &bookings {
        sqlx::query(
            "INSERT INTO booking (id, room_id, title, start_at, end_at, created_by, notes, created_at, google_uid)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(room_id)
        .bind(title)
        .bind(start_at)
        .bind(end_at)
        .bind(created_by)
        .bind(notes)
        .bind(created_at)
        .bind(google_uid)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("copying booking {id}"))?;
    }
    tx.commit().await?;

    tracing::info!(rooms = rooms.len(), bookings = bookings.len(), "Calendar: data copied from PostgreSQL");
    Ok(())
}
