use axum::{Json, extract::State};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::invoice::state::State as InvoiceState;

#[derive(Serialize, ToSchema)]
pub struct OccupancySlot {
    /// Day of week: 1=Monday … 5=Friday
    pub day: i32,
    pub hour: i32,
    pub count: i32,
}

#[derive(Serialize, ToSchema)]
pub struct OccupancyResponse {
    pub slots: Vec<OccupancySlot>,
    /// Maximum count across all slots this week — used for color scaling.
    pub max_value: i32,
}

#[derive(FromRow)]
struct OccupancyRow {
    day: i32,
    hour: i32,
    count: i32,
}

#[utoipa::path(
    get,
    path = "/occupancy",
    tag = "Occupancy",
    responses(
        (status = 200, description = "Hourly occupancy for the current week", body = OccupancyResponse),
    )
)]
pub async fn get_occupancy(
    State(state): State<InvoiceState>,
) -> Result<Json<OccupancyResponse>, AppError> {
    let rows = sqlx::query_as::<_, OccupancyRow>(
        r#"
        SELECT
            CASE EXTRACT(DOW FROM slot_start AT TIME ZONE 'Europe/Paris')::int
                WHEN 0 THEN 7
                ELSE EXTRACT(DOW FROM slot_start AT TIME ZONE 'Europe/Paris')::int
            END                                                             AS day,
            EXTRACT(HOUR FROM slot_start AT TIME ZONE 'Europe/Paris')::int AS hour,
            connected_count                                                 AS count
        FROM portal_occupancy_snapshot
        WHERE slot_start >= date_trunc('week', NOW() AT TIME ZONE 'Europe/Paris') AT TIME ZONE 'Europe/Paris'
          AND slot_start <  date_trunc('week', NOW() AT TIME ZONE 'Europe/Paris') AT TIME ZONE 'Europe/Paris'
                            + INTERVAL '5 days'
        ORDER BY day, hour
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let max_value = rows.iter().map(|r| r.count).max().unwrap_or(0);

    let slots = rows
        .into_iter()
        .map(|r| OccupancySlot { day: r.day, hour: r.hour, count: r.count })
        .collect();

    Ok(Json(OccupancyResponse { slots, max_value }))
}
