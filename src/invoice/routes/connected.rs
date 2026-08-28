use axum::extract::State;
use axum::Json;
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::invoice::state::State as InvoiceState;

#[derive(Serialize, ToSchema)]
pub struct ConnectedAccountUser {
    pub user_id: i32,
    pub username: String,
    pub first_name: String,
    pub voucher_unify_id: String,
    pub macs: Vec<String>,
    pub minutes_remaining: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct ConnectedGuestsResponse {
    pub total: usize,
    pub account_users: Vec<ConnectedAccountUser>,
    pub guest_count: usize,
    pub unknown_count: usize,
}

#[derive(FromRow)]
struct VoucherOwnerRow {
    unify_id: String,
    user_id: i32,
    first_name: String,
    username: String,
    is_guest: bool,
}

#[utoipa::path(
    get,
    path = "/connected",
    tag = "Vouchers",
    responses(
        (status = 200, description = "Currently connected guests", body = ConnectedGuestsResponse),
    )
)]
pub async fn connected_guests(
    State(state): State<InvoiceState>,
) -> Result<Json<ConnectedGuestsResponse>, AppError> {
    let active = state.unify.get_active_guests().await
        .map_err(|e| AppError::External(e.to_string()))?;

    if active.is_empty() {
        return Ok(Json(ConnectedGuestsResponse {
            total: 0,
            account_users: vec![],
            guest_count: 0,
            unknown_count: 0,
        }));
    }

    let unify_ids: Vec<&str> = active.iter().map(|g| g.voucher_id.as_str()).collect();

    let rows = sqlx::query_as::<_, VoucherOwnerRow>(
        r#"
        SELECT
            pv.unify_id,
            b.user_id,
            au.first_name,
            au.username,
            (pgb.guest_token IS NOT NULL) AS is_guest
        FROM portal_voucher pv
        JOIN billjobs_bill b ON b.id = pv.bill_id
        LEFT JOIN auth_user au ON au.id = b.user_id
        LEFT JOIN portal_guest_bill pgb ON pgb.bill_id = b.id
        WHERE pv.unify_id = ANY($1)
        "#,
    )
    .bind(&unify_ids as &[&str])
    .fetch_all(&state.db)
    .await?;

    let owner_map: std::collections::HashMap<&str, &VoucherOwnerRow> =
        rows.iter().map(|r| (r.unify_id.as_str(), r)).collect();

    let mut account_users = Vec::new();
    let mut guest_count = 0usize;
    let mut unknown_count = 0usize;

    for guest in &active {
        match owner_map.get(guest.voucher_id.as_str()) {
            Some(row) if row.is_guest => guest_count += 1,
            Some(row) => account_users.push(ConnectedAccountUser {
                user_id: row.user_id,
                username: row.username.clone(),
                first_name: row.first_name.clone(),
                voucher_unify_id: guest.voucher_id.clone(),
                macs: guest.macs.clone(),
                minutes_remaining: guest.minutes,
            }),
            None => unknown_count += 1,
        }
    }

    let total = active.len();
    Ok(Json(ConnectedGuestsResponse { total, account_users, guest_count, unknown_count }))
}

#[derive(Serialize, ToSchema)]
pub struct OnsitePaymentPresenceResponse {
    pub present: bool,
}

#[utoipa::path(
    get,
    path = "/connected/onsite-payment",
    tag = "Vouchers",
    responses(
        (status = 200, description = "Whether any connected account user has onsite payment enabled", body = OnsitePaymentPresenceResponse),
    )
)]
pub async fn get_onsite_payment_presence(
    State(state): State<InvoiceState>,
) -> Result<Json<OnsitePaymentPresenceResponse>, AppError> {
    let active = state.unify.get_active_guests().await
        .map_err(|e| AppError::External(e.to_string()))?;

    if active.is_empty() {
        return Ok(Json(OnsitePaymentPresenceResponse { present: false }));
    }

    let unify_ids: Vec<&str> = active.iter().map(|g| g.voucher_id.as_str()).collect();

    let present: bool = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM portal_voucher pv
            JOIN billjobs_bill b ON b.id = pv.bill_id
            JOIN portal_user_settings s ON s.user_id = b.user_id
            WHERE pv.unify_id = ANY($1)
              AND s.onsite_payment = true
        )
        "#,
    )
    .bind(&unify_ids as &[&str])
    .fetch_one(&state.db)
    .await?;

    Ok(Json(OnsitePaymentPresenceResponse { present }))
}
