use axum::{
    Json,
    extract::{Path, State},
    response::Response,
};
use chrono::{Datelike, NaiveDate, Utc};
use serde::Serialize;
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::invoice::django_pdf::proxy_bill_pdf;
use crate::invoice::domain::{VoucherStatus, format_code};
use crate::invoice::state::State as InvoiceState;
use crate::invoice::unify::CreateVouchersRequest;
use crate::users::auth::CurrentUser;

#[derive(FromRow)]
struct VoucherCheckRow {
    unify_id: String,
    unify_create_time: i64,
    code: String,
    duration: i32,
    status: String,
    bill_number: String,
    first_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct VoucherStatusResponse {
    pub unify_id: String,
    pub code: String,
    pub duration: i32,
    pub status: String,
}

#[derive(Serialize, ToSchema)]
pub struct VoucherCheckResponse {
    pub data: Vec<VoucherStatusResponse>,
}

#[utoipa::path(
    get,
    path = "/bills/{id}/vouchers/check",
    tag = "Vouchers",
    security(("bearer_auth" = [])),
    params(
        ("id" = i32, Path, description = "Bill ID"),
    ),
    responses(
        (status = 200, description = "Live voucher status from Unify", body = VoucherCheckResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Bill not found or not owned by user"),
    )
)]
pub async fn check_vouchers(
    State(state): State<InvoiceState>,
    user: CurrentUser,
    Path(bill_id): Path<i32>,
) -> Result<Json<VoucherCheckResponse>, AppError> {
    // Verify bill belongs to user and fetch vouchers with note reconstruction data
    let rows = sqlx::query_as::<_, VoucherCheckRow>(
        r#"
        SELECT v.unify_id, v.unify_create_time, v.code, v.duration, v.status,
               b.number AS bill_number, u.first_name
        FROM portal_voucher v
        JOIN billjobs_bill b ON b.id = v.bill_id
        JOIN auth_user u ON u.id = b.user_id
        WHERE v.bill_id = $1 AND b.user_id = $2
        "#,
    )
    .bind(bill_id)
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound);
    }

    // "Revoked" is a terminal, app-initiated state (set by the split feature), not
    // something Unify's live status can confirm or contradict — a revoked voucher is
    // gone from Unify entirely, which would otherwise get misread as generic "Expired".
    // Skip polling/overwriting those; only sync vouchers still tracked as live on Unify.
    let (revoked_rows, pollable_rows): (Vec<_>, Vec<_>) =
        rows.into_iter().partition(|r| r.status == "Revoked");

    // Vouchers on a bill may come from more than one Unify creation batch (e.g. a
    // voucher split creates a fresh batch alongside the original one), so status
    // must be polled per distinct create_time and merged, not just from the first row.
    let note = pollable_rows
        .first()
        .map(|r| format!("{}_{}", r.bill_number, r.first_name))
        .unwrap_or_default();
    let mut batches: HashMap<i64, Vec<String>> = HashMap::new();
    for r in &pollable_rows {
        batches.entry(r.unify_create_time).or_default().push(r.unify_id.clone());
    }

    let mut statuses: HashMap<String, VoucherStatus> = HashMap::new();
    for (create_time, unify_ids) in batches {
        let batch_statuses = state
            .unify
            .get_vouchers_status(create_time, &note, &unify_ids)
            .await
            .map_err(|e| AppError::External(e.to_string()))?;
        statuses.extend(batch_statuses);
    }

    let mut data = Vec::with_capacity(pollable_rows.len() + revoked_rows.len());
    for r in pollable_rows {
        let status = statuses
            .get(&r.unify_id)
            .cloned()
            .unwrap_or(VoucherStatus::Unknown);

        sqlx::query("UPDATE portal_voucher SET status = $1 WHERE unify_id = $2")
            .bind(status.as_str())
            .bind(&r.unify_id)
            .execute(&state.db)
            .await?;

        data.push(VoucherStatusResponse {
            unify_id: r.unify_id,
            code: format_code(&r.code),
            duration: r.duration,
            status: status.as_str().to_string(),
        });
    }
    for r in revoked_rows {
        data.push(VoucherStatusResponse {
            unify_id: r.unify_id,
            code: format_code(&r.code),
            duration: r.duration,
            status: r.status,
        });
    }

    Ok(Json(VoucherCheckResponse { data }))
}

#[utoipa::path(
    get,
    path = "/bills/{id}/pdf",
    tag = "Vouchers",
    security(("bearer_auth" = [])),
    params(
        ("id" = i32, Path, description = "Bill ID"),
    ),
    responses(
        (status = 200, description = "Invoice PDF from Django"),
        (status = 401, description = "Unauthorized or no Django session"),
        (status = 404, description = "Bill not found or not owned by user"),
    )
)]
pub async fn bill_pdf(
    State(state): State<InvoiceState>,
    user: CurrentUser,
    Path(bill_id): Path<i32>,
) -> Result<Response, AppError> {
    // Verify bill ownership
    let exists: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM billjobs_bill WHERE id = $1 AND user_id = $2",
    )
    .bind(bill_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    proxy_bill_pdf(&state, bill_id).await
}

#[derive(FromRow)]
struct RevocableVoucherRow {
    billing_date: NaiveDate,
    kind: String,
}

#[derive(Serialize, ToSchema)]
pub struct RevokeVoucherResponse {
    pub status: String,
}

#[utoipa::path(
    post,
    path = "/bills/{id}/vouchers/{unify_id}/revoke",
    tag = "Vouchers",
    security(("bearer_auth" = [])),
    params(
        ("id" = i32, Path, description = "Bill ID"),
        ("unify_id" = String, Path, description = "Unify voucher ID"),
    ),
    responses(
        (status = 200, description = "Voucher revoked", body = RevokeVoucherResponse),
        (status = 400, description = "Voucher is not eligible for revocation"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Bill or voucher not found, or not owned by user"),
    )
)]
pub async fn revoke_voucher(
    State(state): State<InvoiceState>,
    user: CurrentUser,
    Path((bill_id, unify_id)): Path<(i32, String)>,
) -> Result<Json<RevokeVoucherResponse>, AppError> {
    let row = sqlx::query_as::<_, RevocableVoucherRow>(
        r#"
        SELECT b.billing_date, ps.kind
        FROM portal_voucher v
        JOIN billjobs_bill b ON b.id = v.bill_id
        JOIN billjobs_billline bl ON bl.id = v.billline_id
        JOIN portal_service ps ON ps.external_service_id = bl.service_id
        WHERE v.unify_id = $1 AND v.bill_id = $2 AND b.user_id = $3
        "#,
    )
    .bind(&unify_id)
    .bind(bill_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.kind != "Monthly" {
        return Err(AppError::BadRequest(
            "Seuls les vouchers mensuels peuvent être révoqués".to_string(),
        ));
    }

    let today = Utc::now().date_naive();
    if (row.billing_date.year(), row.billing_date.month()) >= (today.year(), today.month()) {
        return Err(AppError::BadRequest(
            "Seul un voucher d'un mois précédent peut être révoqué".to_string(),
        ));
    }

    state
        .unify
        .revoke_voucher(&unify_id)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    sqlx::query("UPDATE portal_voucher SET status = 'Expired' WHERE unify_id = $1")
        .bind(&unify_id)
        .execute(&state.db)
        .await?;

    Ok(Json(RevokeVoucherResponse { status: "Expired".to_string() }))
}

#[derive(FromRow)]
struct SplittableVoucherRow {
    billline_id: i32,
    duration: i32,
    status: String,
    bill_number: String,
    external_service_id: i32,
    first_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct SplitVoucherResponse {
    pub status: String,
    pub vouchers: Vec<VoucherStatusResponse>,
}

#[utoipa::path(
    post,
    path = "/bills/{id}/vouchers/{unify_id}/split",
    tag = "Vouchers",
    security(("bearer_auth" = [])),
    params(
        ("id" = i32, Path, description = "Bill ID"),
        ("unify_id" = String, Path, description = "Unify voucher ID"),
    ),
    responses(
        (status = 200, description = "Voucher split into two half-duration vouchers", body = SplitVoucherResponse),
        (status = 400, description = "Voucher is not eligible for splitting"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Bill or voucher not found, or not owned by user"),
    )
)]
pub async fn split_voucher(
    State(state): State<InvoiceState>,
    user: CurrentUser,
    Path((bill_id, unify_id)): Path<(i32, String)>,
) -> Result<Json<SplitVoucherResponse>, AppError> {
    let row = sqlx::query_as::<_, SplittableVoucherRow>(
        r#"
        SELECT v.billline_id, v.duration, v.status,
               b.number AS bill_number, bl.service_id AS external_service_id, u.first_name
        FROM portal_voucher v
        JOIN billjobs_bill b ON b.id = v.bill_id
        JOIN billjobs_billline bl ON bl.id = v.billline_id
        JOIN auth_user u ON u.id = b.user_id
        WHERE v.unify_id = $1 AND v.bill_id = $2 AND b.user_id = $3
        "#,
    )
    .bind(&unify_id)
    .bind(bill_id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.duration != 10 || row.status != "Valid" {
        return Err(AppError::BadRequest(
            "Seuls les vouchers valides de 10h peuvent être divisés".to_string(),
        ));
    }

    let note = format!(
        "{}_{}_{}_split_{}",
        row.bill_number, row.external_service_id, row.first_name, unify_id
    );

    let new_vouchers = state
        .unify
        .create_vouchers(CreateVouchersRequest {
            n: 2,
            duration_hours: 5,
            note,
            quota: 2,
        })
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    state
        .unify
        .revoke_voucher(&unify_id)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    let mut tx = state.db.begin().await?;
    let now = Utc::now();

    for nv in &new_vouchers {
        sqlx::query(
            "INSERT INTO portal_voucher (unify_id, bill_id, billline_id, unify_create_time, code, created_at, duration, status) VALUES ($1, $2, $3, $4, $5, $6, $7, 'Valid')",
        )
        .bind(&nv.unify_id)
        .bind(bill_id)
        .bind(row.billline_id)
        .bind(nv.create_time)
        .bind(&nv.code)
        .bind(now)
        .bind(nv.duration)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("UPDATE portal_voucher SET status = 'Revoked' WHERE unify_id = $1")
        .bind(&unify_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(SplitVoucherResponse {
        status: "Revoked".to_string(),
        vouchers: new_vouchers
            .into_iter()
            .map(|nv| VoucherStatusResponse {
                unify_id: nv.unify_id,
                code: format_code(&nv.code),
                duration: nv.duration,
                status: "Valid".to_string(),
            })
            .collect(),
    }))
}
