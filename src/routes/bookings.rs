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
    pub created_by: i32,
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
    security(("bearer_auth" = [])),
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
    _user: CurrentUser,
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

    let room_exists: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM portal_room WHERE id = $1",
    )
    .bind(body.room_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    if room_exists.is_none() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Salle introuvable"}))));
    }

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

    let booking = sqlx::query_as::<_, BookingResponse>(
        "INSERT INTO portal_room_booking (room_id, title, start_at, end_at, created_by, notes) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, room_id, title, start_at, end_at, created_by, notes, created_at",
    )
    .bind(body.room_id)
    .bind(&body.title)
    .bind(body.start_at)
    .bind(body.end_at)
    .bind(user_id)
    .bind(body.notes.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    Ok((StatusCode::CREATED, Json(booking)))
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
    let deleted: Option<i32> = sqlx::query_scalar(
        "DELETE FROM portal_room_booking WHERE id = $1 RETURNING id",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Erreur serveur"}))))?;

    if deleted.is_none() {
        return Err((StatusCode::NOT_FOUND, Json(json!({"error": "Réservation introuvable"}))));
    }

    Ok(Json(json!({})))
}
