use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::AppState;
use crate::auth::CurrentUser;

type ApiResult<T> = Result<T, (StatusCode, Json<serde_json::Value>)>;

#[derive(sqlx::FromRow, Serialize, ToSchema)]
pub struct BookingResponse {
    pub id: i32,
    pub room_id: i32,
    pub title: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub created_by: Option<i32>,
    pub notes: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBookingRequest {
    pub room_id: i32,
    pub title: String,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct ListBookingsQuery {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[utoipa::path(
    get,
    path = "/bookings",
    tag = "Bookings",
    params(
        ("start" = String, Query, description = "Range start (ISO 8601)"),
        ("end" = String, Query, description = "Range end (ISO 8601)"),
    ),
    responses(
        (status = 200, description = "Bookings in range", body = Vec<BookingResponse>),
    )
)]
pub async fn list_bookings(
    State(state): State<AppState>,
    Query(params): Query<ListBookingsQuery>,
) -> ApiResult<Json<Vec<BookingResponse>>> {
    let bookings = sqlx::query_as::<_, BookingResponse>(
        "SELECT id, room_id, title, start_at, end_at, created_by, notes, created_at \
         FROM portal_room_booking \
         WHERE start_at < $2 AND end_at > $1 \
         ORDER BY start_at",
    )
    .bind(params.start)
    .bind(params.end)
    .fetch_all(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    Ok(Json(bookings))
}

#[utoipa::path(
    post,
    path = "/bookings",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    request_body = CreateBookingRequest,
    responses(
        (status = 201, description = "Booking created", body = BookingResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Room already booked for this slot"),
    )
)]
pub async fn create_booking(
    State(state): State<AppState>,
    CurrentUser { id: user_id, .. }: CurrentUser,
    Json(body): Json<CreateBookingRequest>,
) -> ApiResult<(StatusCode, Json<BookingResponse>)> {
    if body.end_at <= body.start_at {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "La fin doit être après le début"}))));
    }

    let room_name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM portal_room WHERE id = $1",
    )
    .bind(body.room_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    let room_name = room_name.ok_or((StatusCode::BAD_REQUEST, Json(json!({"error": "Salle introuvable"}))))?;

    let conflict: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM portal_room_booking \
         WHERE room_id = $1 AND start_at < $3 AND end_at > $2 \
         LIMIT 1",
    )
    .bind(body.room_id)
    .bind(body.start_at)
    .bind(body.end_at)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    if conflict.is_some() {
        return Err((StatusCode::CONFLICT, Json(json!({"error": "Cette salle est déjà réservée sur ce créneau"}))));
    }

    let caldav = match (state.config.google_caldav_enabled, &state.config.google_caldav_email, &state.config.google_caldav_password, &state.config.google_caldav_calendar_id) {
        (true, Some(email), Some(password), Some(calendar_id)) => Some((email.as_str(), password.as_str(), calendar_id.as_str())),
        _ => None,
    };

    let google_uid: Option<String> = if caldav.is_some() {
        Some(uuid::Uuid::new_v4().to_string())
    } else {
        None
    };

    if let (Some(uid), Some((email, password, calendar_id))) = (&google_uid, caldav) {
        let caldav_summary = format!("{} / {}", room_name, body.title);
        let client = crate::caldav::CalDavClient { email, password, calendar_id };
        if let Err(e) = client.create_event(uid, &caldav_summary, body.start_at, body.end_at, body.notes.as_deref().unwrap_or("")).await {
            tracing::error!(uid, error = %e, "CalDAV create event failed — booking will be created without Google Calendar sync");
        }
    }

    let booking = sqlx::query_as::<_, BookingResponse>(
        "INSERT INTO portal_room_booking (room_id, title, start_at, end_at, created_by, notes, google_uid) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id, room_id, title, start_at, end_at, created_by, notes, created_at",
    )
    .bind(body.room_id)
    .bind(&body.title)
    .bind(body.start_at)
    .bind(body.end_at)
    .bind(user_id)
    .bind(body.notes.as_deref().unwrap_or(""))
    .bind(&google_uid)
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    Ok((StatusCode::CREATED, Json(booking)))
}

#[derive(sqlx::FromRow)]
struct DeletedBooking {
    #[allow(dead_code)]
    id: i32,
    google_uid: Option<String>,
}

#[utoipa::path(
    delete,
    path = "/bookings/{id}",
    tag = "Bookings",
    security(("bearer_auth" = [])),
    params(("id" = i32, Path, description = "Booking ID")),
    responses(
        (status = 200, description = "Booking deleted"),
        (status = 404, description = "Booking not found"),
    )
)]
pub async fn delete_booking(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i32>,
) -> ApiResult<Json<serde_json::Value>> {
    let deleted = sqlx::query_as::<_, DeletedBooking>(
        "DELETE FROM portal_room_booking WHERE id = $1 RETURNING id, google_uid",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    let deleted = deleted.ok_or((StatusCode::NOT_FOUND, Json(json!({"error": "Réservation introuvable"}))))?;

    if let (true, Some(uid), Some(email), Some(password), Some(calendar_id)) = (
        state.config.google_caldav_enabled,
        &deleted.google_uid,
        &state.config.google_caldav_email,
        &state.config.google_caldav_password,
        &state.config.google_caldav_calendar_id,
    ) {
        let client = crate::caldav::CalDavClient { email, password, calendar_id };
        if let Err(e) = client.delete_event(uid).await {
            tracing::error!(uid, error = %e, "CalDAV delete event failed");
        }
    }

    Ok(Json(json!({})))
}
