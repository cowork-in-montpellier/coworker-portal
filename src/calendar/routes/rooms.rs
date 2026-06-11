use axum::{Json, extract::{Path, State}, response::IntoResponse};
use chrono::Utc;
use icalendar::{Calendar, Component, Event, EventLike};
use serde::Serialize;
use serde_json::json;
use utoipa::ToSchema;
use axum::http::StatusCode;

use crate::calendar::state::State as CalendarState;

type ApiResult<T> = Result<T, (StatusCode, Json<serde_json::Value>)>;

#[derive(sqlx::FromRow, Serialize, ToSchema)]
pub struct RoomResponse {
    pub id: i32,
    pub name: String,
    pub color: String,
}

pub(crate) struct IcalResponse(String, chrono::DateTime<Utc>);

impl IntoResponse for IcalResponse {
    fn into_response(self) -> axum::response::Response {
        let last_modified = self.1.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        (
            [
                (axum::http::header::CONTENT_TYPE, "text/calendar; charset=utf-8".to_string()),
                (axum::http::header::LAST_MODIFIED, last_modified),
            ],
            self.0,
        )
            .into_response()
    }
}

#[derive(sqlx::FromRow)]
struct BookingRow {
    id: i32,
    title: String,
    start_at: chrono::DateTime<Utc>,
    end_at: chrono::DateTime<Utc>,
    notes: String,
    created_at: chrono::DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct BookingWithRoomRow {
    id: i32,
    title: String,
    room_name: String,
    start_at: chrono::DateTime<Utc>,
    end_at: chrono::DateTime<Utc>,
    notes: String,
    created_at: chrono::DateTime<Utc>,
}

#[utoipa::path(
    get,
    path = "/rooms",
    tag = "Rooms",
    responses(
        (status = 200, description = "List of rooms", body = Vec<RoomResponse>),
    )
)]
pub async fn list_rooms(
    State(state): State<CalendarState>,
) -> ApiResult<Json<Vec<RoomResponse>>> {
    let rooms = sqlx::query_as::<_, RoomResponse>(
        "SELECT id, name, color FROM portal_room ORDER BY id",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    Ok(Json(rooms))
}

#[utoipa::path(
    get,
    path = "/rooms/{id}/calendar.ics",
    tag = "Rooms",
    params(("id" = i32, Path, description = "Room ID")),
    responses(
        (status = 200, description = "iCalendar feed for the room"),
        (status = 404, description = "Room not found"),
    )
)]
pub async fn room_calendar(
    State(state): State<CalendarState>,
    Path(id): Path<i32>,
) -> ApiResult<IcalResponse> {
    let room = sqlx::query_as::<_, RoomResponse>(
        "SELECT id, name, color FROM portal_room WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?
    .ok_or((StatusCode::NOT_FOUND, Json(json!({"error": "Salle introuvable"}))))?;

    let bookings = sqlx::query_as::<_, BookingRow>(
        "SELECT id, title, start_at, end_at, notes, created_at FROM portal_room_booking WHERE room_id = $1 ORDER BY start_at",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    let last_modified = bookings.iter().map(|b| b.created_at).max().unwrap_or_else(Utc::now);

    let mut calendar = Calendar::new();
    calendar.name(&format!("Cowork'in Montpellier — {}", &room.name));
    for b in bookings {
        let event = Event::new()
            .summary(&b.title)
            .uid(&format!("booking-{}", b.id))
            .starts(b.start_at)
            .ends(b.end_at)
            .description(&b.notes)
            .done();
        calendar.push(event);
    }

    Ok(IcalResponse(calendar.to_string(), last_modified))
}

#[utoipa::path(
    get,
    path = "/calendar.ics",
    tag = "Rooms",
    responses(
        (status = 200, description = "iCalendar feed for all rooms"),
    )
)]
pub async fn all_calendar(
    State(state): State<CalendarState>,
) -> ApiResult<IcalResponse> {
    let bookings = sqlx::query_as::<_, BookingWithRoomRow>(
        "SELECT b.id, b.title, r.name AS room_name, b.start_at, b.end_at, b.notes, b.created_at \
         FROM portal_room_booking b \
         JOIN portal_room r ON r.id = b.room_id \
         ORDER BY b.start_at",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    let last_modified = bookings.iter().map(|b| b.created_at).max().unwrap_or_else(Utc::now);

    let mut calendar = Calendar::new();
    calendar.name("Cowork'in Montpellier — Toutes les salles");
    for b in bookings {
        let summary = format!("{} / {}", b.room_name, b.title);
        let event = Event::new()
            .summary(&summary)
            .uid(&format!("booking-{}", b.id))
            .starts(b.start_at)
            .ends(b.end_at)
            .description(&b.notes)
            .done();
        calendar.push(event);
    }

    Ok(IcalResponse(calendar.to_string(), last_modified))
}
