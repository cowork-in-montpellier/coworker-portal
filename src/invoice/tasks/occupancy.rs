use chrono::{Timelike, Utc};
use chrono_tz::Europe::Paris;

use crate::invoice::state::State;

pub async fn run(state: &State) {
    tracing::info!("Occupancy snapshot: starting");

    // The job fires at the end of the slot (e.g. 09:00 = records 08:00–09:00).
    // slot_start = current Paris hour minus 1, truncated to the whole hour, stored as UTC.
    let now_paris = Utc::now().with_timezone(&Paris);
    let slot_paris = (now_paris - chrono::Duration::hours(1))
        .with_minute(0).unwrap()
        .with_second(0).unwrap()
        .with_nanosecond(0).unwrap();
    let slot_start = slot_paris.with_timezone(&Utc);

    tracing::info!(slot_start = %slot_start, "Occupancy snapshot: querying active guests");

    let guests = match state.unify.get_active_guests().await {
        Ok(g) => g,
        Err(e) => {
            tracing::error!(error = %e, "Occupancy snapshot: Unify query failed");
            return;
        }
    };

    let count = guests.len() as i32;
    tracing::info!(slot_start = %slot_start, count, "Occupancy snapshot: storing");

    match sqlx::query(
        "INSERT INTO portal_occupancy_snapshot (slot_start, connected_count)
         VALUES ($1, $2)
         ON CONFLICT (slot_start) DO UPDATE SET connected_count = EXCLUDED.connected_count",
    )
    .bind(slot_start)
    .bind(count)
    .execute(&state.db)
    .await
    {
        Ok(_) => tracing::info!(slot_start = %slot_start, count, "Occupancy snapshot: done"),
        Err(e) => tracing::error!(error = %e, "Occupancy snapshot: DB insert failed"),
    }
}
