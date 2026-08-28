use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::invoice::django_pdf::proxy_bill_pdf;
use crate::invoice::domain::{PaymentMethod, Service, VoucherStatus, format_code, next_bill_number, resolve_voucher_params};
use crate::invoice::repository;
use crate::invoice::routes::vouchers::{VoucherCheckResponse, VoucherStatusResponse};
use crate::invoice::state::State as InvoiceState;
use crate::invoice::sumup::CheckoutStatus;
use crate::invoice::unify::CreateVouchersRequest;

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Serialize, ToSchema)]
pub struct GuestServicesResponse {
    pub data: Vec<Service>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct GuestVoucherResponse {
    pub unify_id: String,
    pub code: String,
    pub duration: i32,
    pub status: String,
    pub active_days_count: i32,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct GuestBillLineResponse {
    pub service_name: String,
    pub quantity: i32,
    pub vouchers: Vec<GuestVoucherResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct GuestBillResponse {
    pub guest_token: String,
    pub bill_id: i32,
    pub bill_number: String,
    pub date: String,
    pub amount: f64,
    pub is_paid: bool,
    pub payment_method: PaymentMethod,
    pub checkout_failed: bool,
    pub lines: Vec<GuestBillLineResponse>,
    /// SumUp hosted checkout URL, present only on create when card checkout succeeds.
    pub payment_url: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PaymentStatusResponse {
    pub paid: bool,
    pub amount: Option<f64>,
}

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct CreateGuestBillLineRequest {
    pub service_id: i32,
    #[serde(default = "default_quantity")]
    pub quantity: i32,
}
fn default_quantity() -> i32 { 1 }

#[derive(Deserialize, ToSchema)]
pub struct CreateGuestBillRequest {
    pub lines: Vec<CreateGuestBillLineRequest>,
    /// Guest's email address — required, used to send the order confirmation.
    pub guest_email: String,
    /// Optional customer name — prepended to billing_address so it appears in the Django-generated PDF.
    pub billing_name: Option<String>,
    /// Optional billing address lines.
    pub billing_address: Option<String>,
    /// "card" → trigger SumUp checkout; "on_site" or absent → skip SumUp.
    pub payment_method: Option<PaymentMethod>,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/guest/services",
    tag = "Guest",
    responses(
        (status = 200, description = "List of guest-available services", body = GuestServicesResponse),
    )
)]
pub async fn list_guest_services(
    State(state): State<InvoiceState>,
) -> Result<Json<GuestServicesResponse>, AppError> {
    let data = repository::list_guest_available_services(&state.db).await?;
    Ok(Json(GuestServicesResponse { data }))
}

#[utoipa::path(
    post,
    path = "/guest/bills",
    tag = "Guest",
    request_body = CreateGuestBillRequest,
    responses(
        (status = 200, description = "Guest bill created with vouchers", body = GuestBillResponse),
        (status = 404, description = "Service not found or not guest-available"),
    )
)]
pub async fn create_guest_bill(
    State(state): State<InvoiceState>,
    Json(body): Json<CreateGuestBillRequest>,
) -> Result<Json<GuestBillResponse>, AppError> {
    if body.lines.is_empty() {
        return Err(AppError::BadRequest("At least one line is required".into()));
    }
    if body.guest_email.is_empty() || !body.guest_email.contains('@') {
        return Err(AppError::BadRequest("A valid email address is required".into()));
    }

    let now = Utc::now();

    // 1. Fetch all services (must all be guest-available); validate quantity >= 1
    let mut service_lines: Vec<(Service, i32)> = Vec::with_capacity(body.lines.len());
    for line_req in &body.lines {
        if line_req.quantity < 1 {
            return Err(AppError::BadRequest(format!("Quantity must be at least 1 (got {})", line_req.quantity)));
        }
        let service = repository::fetch_guest_service(&state.db, line_req.service_id).await?;
        service_lines.push((service, line_req.quantity));
    }

    // 2. Compute total amount (price × quantity per line)
    let total_amount: f64 = service_lines.iter().map(|(s, q)| s.price * (*q as f64)).sum();

    // 3. Build billing address — name prepended if provided
    let fallback_address = state
        .billing_directory
        .billing_address(state.config.guest_user_id)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    let address_body = body.billing_address.as_deref().unwrap_or(&fallback_address);
    let billing_address = match body.billing_name.as_deref().filter(|n| !n.is_empty()) {
        Some(name) => format!("{}\n{}", name, address_body),
        None => address_body.to_string(),
    };

    // 4. Generate guest token
    let guest_token = Uuid::new_v4();

    // 5. Begin transaction
    let mut tx = state.db.begin().await?;

    // 6. Acquire advisory lock + compute next bill number
    sqlx::query("SELECT pg_advisory_xact_lock(42)")
        .execute(&mut *tx)
        .await?;
    let last_number: Option<String> =
        sqlx::query_scalar("SELECT number FROM billjobs_bill ORDER BY id DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;
    let number = next_bill_number(last_number.as_deref(), now.date_naive());

    tracing::info!(number = &number, lines = service_lines.len(), guest_token = %guest_token, "Creating guest bill");

    // 7. Insert bill
    let bill_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO billjobs_bill
            (number, user_id, billing_date, amount, issuer_address, billing_address, "isPaid")
        VALUES ($1, $2, $3, $4, $5, $6, false)
        RETURNING id
        "#,
    )
    .bind(&number)
    .bind(state.config.guest_user_id)
    .bind(now.date_naive())
    .bind(total_amount)
    .bind(&state.config.issuer_address)
    .bind(&billing_address)
    .fetch_one(&mut *tx)
    .await?;

    // 8. Link bill to guest token (default to on_site; updated below if card checkout succeeds)
    sqlx::query("INSERT INTO portal_guest_bill (guest_token, bill_id, payment_method, guest_email) VALUES ($1, $2, $3, $4)")
        .bind(guest_token)
        .bind(bill_id)
        .bind(PaymentMethod::OnSite.as_str())
        .bind(&body.guest_email)
        .execute(&mut *tx)
        .await?;

    // 9. For each line: insert bill line, provision vouchers, persist vouchers
    let note = format!("{}_Guest", number);
    let mut response_lines: Vec<GuestBillLineResponse> = Vec::with_capacity(service_lines.len());

    for (service, quantity) in &service_lines {
        let (voucher_count, duration_hours) = resolve_voucher_params(&service.voucher_spec, now);
        let line_total = service.price * (*quantity as f64);
        let total_vouchers = voucher_count * quantity;

        let billline_id: i32 = sqlx::query_scalar(
            "INSERT INTO billjobs_billline (bill_id, service_id, quantity, total, note) VALUES ($1, $2, $3, $4, '') RETURNING id",
        )
        .bind(bill_id)
        .bind(service.external_service_id)
        .bind(*quantity as i16)
        .bind(line_total)
        .fetch_one(&mut *tx)
        .await?;

        let unify_vouchers = state
            .unify
            .create_vouchers(CreateVouchersRequest {
                n: total_vouchers,
                duration_hours,
                note: note.clone(),
                quota: 2,
            })
            .await
            .map_err(|e| AppError::External(e.to_string()))?;

        let mut line_vouchers = Vec::new();
        for uv in &unify_vouchers {
            sqlx::query(
                "INSERT INTO portal_voucher (unify_id, bill_id, billline_id, unify_create_time, code, created_at, duration, status) VALUES ($1, $2, $3, $4, $5, $6, $7, 'Valid')",
            )
            .bind(&uv.unify_id)
            .bind(bill_id)
            .bind(billline_id)
            .bind(uv.create_time)
            .bind(&uv.code)
            .bind(now)
            .bind(uv.duration)
            .execute(&mut *tx)
            .await?;

            line_vouchers.push(GuestVoucherResponse {
                unify_id: uv.unify_id.clone(),
                code: format_code(&uv.code),
                duration: uv.duration,
                status: VoucherStatus::Valid.as_str().to_string(),
                active_days_count: 0,
            });
        }

        response_lines.push(GuestBillLineResponse {
            service_name: service.name.clone(),
            quantity: *quantity,
            vouchers: line_vouchers,
        });
    }

    // 10. Commit
    tx.commit().await?;

    // 11. Optionally create a SumUp hosted checkout when guest explicitly chose card payment
    let wants_card = matches!(body.payment_method, Some(PaymentMethod::Card));
    let (payment_url, resolved_method, checkout_failed) = if wants_card && state.sumup.is_some() {
        match provision_sumup_checkout(&state, guest_token, &number, total_amount).await {
            Ok(url) => (Some(url), PaymentMethod::Card, false),
            Err(e) => {
                tracing::error!(error = %e, bill_number = %number, "SumUp checkout creation failed — falling back to on-site");
                if let Err(e) = sqlx::query(
                    "UPDATE portal_guest_bill SET checkout_failed = true WHERE guest_token = $1",
                )
                .bind(guest_token)
                .execute(&state.db)
                .await
                {
                    tracing::error!(error = %e, "Failed to set checkout_failed");
                }
                (None, PaymentMethod::OnSite, true)
            }
        }
    } else {
        (None, PaymentMethod::OnSite, false)
    };

    // 12. Send confirmation email (non-blocking — failure does not fail the request)
    if let Some(smtp) = state.smtp.clone() {
        let to = body.guest_email.clone();
        let summary_url = format!("{}/buy/summary/{}", state.config.app_base_url, guest_token);
        let body_text = build_confirmation_body(&number, &now.date_naive().to_string(), total_amount, &response_lines, &summary_url);
        tokio::spawn(async move {
            if let Err(e) = crate::users::email::send_smtp_email(&smtp, &to, "Commande confirmée", body_text).await {
                tracing::warn!(error = %e, "Failed to send guest order confirmation email");
            }
        });
    }

    Ok(Json(GuestBillResponse {
        guest_token: guest_token.to_string(),
        bill_id,
        bill_number: number,
        date: now.date_naive().to_string(),
        amount: total_amount,
        is_paid: false,
        payment_method: resolved_method,
        checkout_failed,
        lines: response_lines,
        payment_url,
    }))
}

fn build_confirmation_body(
    bill_number: &str,
    date: &str,
    amount: f64,
    lines: &[GuestBillLineResponse],
    summary_url: &str,
) -> String {
    let mut detail = String::new();
    for line in lines {
        let voucher_count = line.vouchers.len();
        let duration = line.vouchers.first().map(|v| v.duration).unwrap_or(0);
        if line.quantity > 1 {
            detail.push_str(&format!(
                "  - {}× {} ({} voucher{}, {}h chacun)\n",
                line.quantity,
                line.service_name,
                voucher_count,
                if voucher_count > 1 { "s" } else { "" },
                duration,
            ));
        } else {
            detail.push_str(&format!(
                "  - {} ({} voucher{}, {}h)\n",
                line.service_name,
                voucher_count,
                if voucher_count > 1 { "s" } else { "" },
                duration,
            ));
        }
    }
    format!(
        "Bonjour,\n\nVotre commande a bien été confirmée.\n\nFacture : {bill_number}\nDate    : {date}\nMontant : {amount:.2} €\n\nDétail :\n{detail}\nRetrouvez vos vouchers et le suivi de votre commande ici :\n{summary_url}\n\nMerci pour votre confiance !\n"
    )
}

#[utoipa::path(
    get,
    path = "/guest/bills/{token}",
    tag = "Guest",
    params(
        ("token" = String, Path, description = "Guest token UUID"),
    ),
    responses(
        (status = 200, description = "Guest bill details", body = GuestBillResponse),
        (status = 404, description = "Bill not found"),
    )
)]
pub async fn get_guest_bill(
    State(state): State<InvoiceState>,
    Path(token): Path<Uuid>,
) -> Result<Json<GuestBillResponse>, AppError> {
    #[derive(FromRow)]
    struct GuestBillRow {
        id: i32,
        number: String,
        billing_date: chrono::NaiveDate,
        amount: f64,
        is_paid: bool,
        payment_method: String,
        checkout_failed: bool,
    }

    let row = sqlx::query_as::<_, GuestBillRow>(
        r#"
        SELECT b.id, b.number, b.billing_date, b.amount, b."isPaid" AS is_paid,
               gb.payment_method, gb.checkout_failed
        FROM billjobs_bill b
        JOIN portal_guest_bill gb ON gb.bill_id = b.id
        WHERE gb.guest_token = $1
        "#,
    )
    .bind(token)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let lines = fetch_guest_bill_lines(&state.db, row.id).await?;

    Ok(Json(GuestBillResponse {
        guest_token: token.to_string(),
        bill_id: row.id,
        bill_number: row.number,
        date: row.billing_date.to_string(),
        amount: row.amount,
        is_paid: row.is_paid,
        payment_method: PaymentMethod::from(row.payment_method.as_str()),
        checkout_failed: row.checkout_failed,
        lines,
        payment_url: None,
    }))
}

async fn fetch_guest_bill_lines(db: &sqlx::PgPool, bill_id: i32) -> Result<Vec<GuestBillLineResponse>, AppError> {
    #[derive(FromRow)]
    struct LineRow {
        line_id: i32,
        service_name: Option<String>,
        quantity: i32,
    }

    #[derive(FromRow)]
    struct VRow {
        billline_id: i32,
        unify_id: String,
        code: String,
        duration: i32,
        status: String,
        active_days_count: i32,
    }

    let line_rows = sqlx::query_as::<_, LineRow>(
        r#"
        SELECT bl.id AS line_id, bl.quantity::int4 AS quantity, s.name AS service_name
        FROM billjobs_billline bl
        LEFT JOIN portal_service s ON s.external_service_id = bl.service_id
        WHERE bl.bill_id = $1
        "#,
    )
    .bind(bill_id)
    .fetch_all(db)
    .await?;

    let voucher_rows = sqlx::query_as::<_, VRow>(
        "SELECT billline_id, unify_id, code, duration, status, cardinality(active_days) AS active_days_count FROM portal_voucher WHERE bill_id = $1",
    )
    .bind(bill_id)
    .fetch_all(db)
    .await?;

    let mut vouchers_by_line: std::collections::HashMap<i32, Vec<GuestVoucherResponse>> = std::collections::HashMap::new();
    for v in voucher_rows {
        vouchers_by_line.entry(v.billline_id).or_default().push(GuestVoucherResponse {
            unify_id: v.unify_id,
            code: format_code(&v.code),
            duration: v.duration,
            status: v.status,
            active_days_count: v.active_days_count,
        });
    }

    Ok(line_rows.into_iter().map(|l| GuestBillLineResponse {
        service_name: l.service_name.unwrap_or_default(),
        quantity: l.quantity,
        vouchers: vouchers_by_line.remove(&l.line_id).unwrap_or_default(),
    }).collect())
}

#[utoipa::path(
    get,
    path = "/guest/bills/{token}/vouchers/check",
    tag = "Guest",
    params(
        ("token" = String, Path, description = "Guest token UUID"),
    ),
    responses(
        (status = 200, description = "Live voucher status from Unify", body = VoucherCheckResponse),
        (status = 404, description = "Bill not found"),
    )
)]
pub async fn check_guest_vouchers(
    State(state): State<InvoiceState>,
    Path(token): Path<Uuid>,
) -> Result<Json<VoucherCheckResponse>, AppError> {
    #[derive(FromRow)]
    struct VoucherCheckRow {
        unify_id: String,
        unify_create_time: i64,
        code: String,
        duration: i32,
        bill_number: String,
    }

    let rows = sqlx::query_as::<_, VoucherCheckRow>(
        r#"
        SELECT v.unify_id, v.unify_create_time, v.code, v.duration,
               b.number AS bill_number
        FROM portal_voucher v
        JOIN billjobs_bill b ON b.id = v.bill_id
        JOIN portal_guest_bill gb ON gb.bill_id = b.id
        WHERE gb.guest_token = $1
        "#,
    )
    .bind(token)
    .fetch_all(&state.db)
    .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound);
    }

    let unify_ids: Vec<String> = rows.iter().map(|r| r.unify_id.clone()).collect();
    let create_time = rows[0].unify_create_time;
    let note = format!("{}_Guest", rows[0].bill_number);

    let statuses = state
        .unify
        .get_vouchers_status(create_time, &note, &unify_ids)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    let mut data = Vec::with_capacity(rows.len());
    for r in rows {
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

    Ok(Json(VoucherCheckResponse { data }))
}

#[derive(Deserialize)]
pub struct SumUpWebhookPayload {
    pub id: String,
}

#[utoipa::path(
    post,
    path = "/guest/payment/webhook",
    tag = "Guest",
    responses(
        (status = 200, description = "Webhook received"),
    )
)]
pub async fn guest_payment_webhook(
    State(state): State<InvoiceState>,
    Json(body): Json<SumUpWebhookPayload>,
) -> StatusCode {
    let checkout_id = body.id.clone();
    tokio::spawn(async move {
        let Some(sumup) = &state.sumup else {
            tracing::warn!(checkout_id, "Received SumUp webhook but SumUp is disabled");
            return;
        };

        let status = match sumup.get_checkout(&checkout_id).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(checkout_id, error = %e, "Failed to verify SumUp checkout status");
                return;
            }
        };

        tracing::info!(checkout_id, ?status, "SumUp webhook: checkout status verified");

        let bill_id: Option<i32> = sqlx::query_scalar(
            "SELECT bill_id FROM portal_guest_bill WHERE sumup_checkout_id = $1",
        )
        .bind(&checkout_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        let Some(bill_id) = bill_id else {
            tracing::warn!(checkout_id, "SumUp webhook: no bill found for checkout_id");
            return;
        };

        match status {
            CheckoutStatus::Paid => {
                if let Err(e) = sqlx::query(r#"UPDATE billjobs_bill SET "isPaid" = true WHERE id = $1"#)
                    .bind(bill_id)
                    .execute(&state.db)
                    .await
                {
                    tracing::error!(bill_id, error = %e, "SumUp webhook: failed to mark bill as paid");
                } else {
                    tracing::info!(bill_id, checkout_id, "SumUp webhook: bill marked as paid");
                }
            }
            CheckoutStatus::Failed => {
                tracing::info!(bill_id, checkout_id, "SumUp webhook: payment failed — revoking vouchers and cleaning up");

                let unify_ids: Vec<String> = sqlx::query_scalar(
                    "SELECT unify_id FROM portal_voucher WHERE bill_id = $1",
                )
                .bind(bill_id)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();

                for unify_id in &unify_ids {
                    if let Err(e) = state.unify.revoke_voucher(unify_id).await {
                        tracing::error!(unify_id, error = %e, "SumUp webhook: failed to revoke Unify voucher");
                    }
                }

                // Delete guest_bill link first (no FK cascade), then bill (cascades portal_voucher)
                let _ = sqlx::query("DELETE FROM portal_guest_bill WHERE sumup_checkout_id = $1")
                    .bind(&checkout_id)
                    .execute(&state.db)
                    .await;
                let _ = sqlx::query("DELETE FROM billjobs_bill WHERE id = $1")
                    .bind(bill_id)
                    .execute(&state.db)
                    .await;

                tracing::info!(bill_id, checkout_id, vouchers = unify_ids.len(), "SumUp webhook: cleanup complete");
            }
            CheckoutStatus::Pending => {
                tracing::debug!(checkout_id, "SumUp webhook: checkout still pending");
            }
        }
    });

    StatusCode::OK
}

#[utoipa::path(
    get,
    path = "/guest/bills/{token}/payment-status",
    tag = "Guest",
    params(
        ("token" = String, Path, description = "Guest token UUID"),
    ),
    responses(
        (status = 200, description = "Payment status for an on-site bill", body = PaymentStatusResponse),
        (status = 404, description = "Bill not found"),
    )
)]
pub async fn get_guest_payment_status(
    State(state): State<InvoiceState>,
    Path(token): Path<Uuid>,
) -> Result<Json<PaymentStatusResponse>, AppError> {
    #[derive(FromRow)]
    struct StatusRow {
        id: i32,
        number: String,
        is_paid: bool,
    }

    let row = sqlx::query_as::<_, StatusRow>(
        r#"
        SELECT b.id, b.number, b."isPaid" AS is_paid
        FROM billjobs_bill b
        JOIN portal_guest_bill gb ON gb.bill_id = b.id
        WHERE gb.guest_token = $1
        "#,
    )
    .bind(token)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.is_paid {
        return Ok(Json(PaymentStatusResponse { paid: true, amount: None }));
    }

    let Some(sumup) = &state.sumup else {
        return Ok(Json(PaymentStatusResponse { paid: false, amount: None }));
    };

    let oldest_time = Utc::now() - chrono::Duration::minutes(10);
    let transactions = sumup
        .get_recent_transactions(oldest_time)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    let matched = transactions.into_iter().find(|t| {
        t.description
            .as_deref()
            .map(|d| d.trim().eq_ignore_ascii_case(&row.number))
            .unwrap_or(false)
    });

    if let Some(tx) = matched {
        if let Err(e) = sqlx::query(r#"UPDATE billjobs_bill SET "isPaid" = true WHERE id = $1"#)
            .bind(row.id)
            .execute(&state.db)
            .await
        {
            tracing::error!(bill_id = row.id, error = %e, "Failed to mark bill as paid via SumUp polling");
        }
        Ok(Json(PaymentStatusResponse { paid: true, amount: Some(tx.amount) }))
    } else {
        Ok(Json(PaymentStatusResponse { paid: false, amount: None }))
    }
}

async fn provision_sumup_checkout(
    state: &InvoiceState,
    guest_token: Uuid,
    bill_number: &str,
    amount: f64,
) -> anyhow::Result<String> {
    let sumup = state.sumup.as_ref()
        .ok_or_else(|| anyhow::anyhow!("SumUp is not configured"))?;
    let redirect = format!("{}/buy/summary/{}", state.config.app_base_url, guest_token);
    let webhook = format!("{}/api/guest/payment/webhook", state.config.app_base_url);
    let created = sumup.create_checkout(&guest_token.to_string(), bill_number, amount, &redirect, &webhook).await?;
    if let Err(e) = sqlx::query(
        "UPDATE portal_guest_bill SET sumup_checkout_id = $1, payment_method = $2, checkout_failed = false WHERE guest_token = $3",
    )
    .bind(&created.checkout_id)
    .bind(PaymentMethod::Card.as_str())
    .bind(guest_token)
    .execute(&state.db)
    .await
    {
        tracing::error!(error = %e, "Failed to store SumUp checkout_id");
    }
    Ok(created.checkout_url)
}

#[derive(Serialize, ToSchema)]
pub struct SwitchToCardResponse {
    pub payment_url: String,
}

#[utoipa::path(
    post,
    path = "/guest/bills/{token}/checkout",
    tag = "Guest",
    params(
        ("token" = String, Path, description = "Guest token UUID"),
    ),
    responses(
        (status = 200, description = "SumUp hosted checkout URL created", body = SwitchToCardResponse),
        (status = 400, description = "Bill already paid or card payment not available"),
        (status = 404, description = "Bill not found"),
    )
)]
pub async fn switch_to_card_checkout(
    State(state): State<InvoiceState>,
    Path(token): Path<Uuid>,
) -> Result<Json<SwitchToCardResponse>, AppError> {
    #[derive(FromRow)]
    struct BillRow {
        number: String,
        amount: f64,
        is_paid: bool,
        payment_method: String,
    }

    let row = sqlx::query_as::<_, BillRow>(
        r#"
        SELECT b.number, b.amount, b."isPaid" AS is_paid, gb.payment_method
        FROM billjobs_bill b
        JOIN portal_guest_bill gb ON gb.bill_id = b.id
        WHERE gb.guest_token = $1
        "#,
    )
    .bind(token)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.is_paid {
        return Err(AppError::BadRequest("Bill is already paid".into()));
    }
    if PaymentMethod::from(row.payment_method.as_str()) == PaymentMethod::Card {
        return Err(AppError::BadRequest("Bill already has a card checkout in progress".into()));
    }
    if state.sumup.is_none() {
        return Err(AppError::BadRequest("Card payment not available".into()));
    }

    let payment_url = provision_sumup_checkout(&state, token, &row.number, row.amount)
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    Ok(Json(SwitchToCardResponse { payment_url }))
}

#[utoipa::path(
    get,
    path = "/guest/bills/{token}/pdf",
    tag = "Guest",
    params(
        ("token" = String, Path, description = "Guest token UUID"),
    ),
    responses(
        (status = 200, description = "Invoice PDF from Django"),
        (status = 404, description = "Bill not found"),
        (status = 502, description = "Django PDF generation failed"),
    )
)]
pub async fn guest_bill_pdf(
    State(state): State<InvoiceState>,
    Path(token): Path<Uuid>,
) -> Result<Response, AppError> {
    // Look up bill_id by guest_token
    let bill_id: i32 = sqlx::query_scalar(
        "SELECT bill_id FROM portal_guest_bill WHERE guest_token = $1",
    )
    .bind(token)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    proxy_bill_pdf(&state, bill_id).await
}
