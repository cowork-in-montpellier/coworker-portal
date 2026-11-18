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
use crate::users::auth::CurrentUser;
use crate::invoice::domain::{
    PaymentMethod, Service, VoucherStatus, format_code, next_bill_number,
    next_guest_order_number, resolve_voucher_params,
};
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
    pub payment_url: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct GuestOrderResponse {
    pub order_token: String,
    pub provisional_number: String,
    pub payment_method: PaymentMethod,
    pub total_amount: f64,
    pub checkout_failed: bool,
    pub guest_token: Option<String>,
    pub payment_url: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct GuestOrderPaymentStatusResponse {
    pub paid: bool,
    pub guest_token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PendingOrderItem {
    pub order_token: String,
    pub provisional_number: String,
    pub total_amount: f64,
    pub created_at: String,
    pub guest_email: String,
}

#[derive(Serialize, ToSchema)]
pub struct PendingOrdersResponse {
    pub data: Vec<PendingOrderItem>,
}

#[derive(Serialize, ToSchema)]
pub struct ValidateOrderResponse {
    pub guest_token: String,
}

// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct CreateGuestOrderLineRequest {
    pub service_id: i32,
    #[serde(default = "default_quantity")]
    pub quantity: i32,
}
fn default_quantity() -> i32 { 1 }

#[derive(Deserialize, ToSchema)]
pub struct CreateGuestOrderRequest {
    pub lines: Vec<CreateGuestOrderLineRequest>,
    pub guest_email: String,
    pub billing_name: Option<String>,
    pub billing_address: Option<String>,
    /// "card" → trigger SumUp checkout; "on_site" or absent → skip SumUp.
    pub payment_method: Option<PaymentMethod>,
}

#[derive(Deserialize)]
pub struct SumUpWebhookPayload {
    pub id: String,
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
    path = "/guest/orders",
    tag = "Guest",
    request_body = CreateGuestOrderRequest,
    responses(
        (status = 200, description = "Guest pending order created", body = GuestOrderResponse),
        (status = 404, description = "Service not found or not guest-available"),
    )
)]
pub async fn create_guest_order(
    State(state): State<InvoiceState>,
    Json(body): Json<CreateGuestOrderRequest>,
) -> Result<Json<GuestOrderResponse>, AppError> {
    if body.lines.is_empty() {
        return Err(AppError::BadRequest("At least one line is required".into()));
    }
    if body.guest_email.is_empty() || !body.guest_email.contains('@') {
        return Err(AppError::BadRequest("A valid email address is required".into()));
    }

    let now = Utc::now();

    let mut service_lines: Vec<(Service, i32)> = Vec::with_capacity(body.lines.len());
    for line_req in &body.lines {
        if line_req.quantity < 1 {
            return Err(AppError::BadRequest(format!("Quantity must be at least 1 (got {})", line_req.quantity)));
        }
        let service = repository::fetch_guest_service(&state.db, line_req.service_id).await?;
        service_lines.push((service, line_req.quantity));
    }

    let total_amount: f64 = service_lines.iter().map(|(s, q)| s.price * (*q as f64)).sum();

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

    let mut tx = state.db.begin().await?;

    sqlx::query("SELECT pg_advisory_xact_lock(43)")
        .execute(&mut *tx)
        .await?;

    let last_provisional: Option<String> = sqlx::query_scalar(
        "SELECT provisional_number FROM portal_guest_pending_order ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let provisional_number = next_guest_order_number(last_provisional.as_deref(), now.date_naive());

    let order_token: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO portal_guest_pending_order
            (provisional_number, payment_method, guest_email, billing_address, total_amount)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING order_token
        "#,
    )
    .bind(&provisional_number)
    .bind(PaymentMethod::OnSite.as_str())
    .bind(&body.guest_email)
    .bind(&billing_address)
    .bind(total_amount)
    .fetch_one(&mut *tx)
    .await?;

    for (service, quantity) in &service_lines {
        sqlx::query(
            "INSERT INTO portal_guest_pending_order_line (order_token, service_id, quantity) VALUES ($1, $2, $3)",
        )
        .bind(order_token)
        .bind(service.id)
        .bind(quantity)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    tracing::info!(
        %order_token,
        provisional_number = &provisional_number,
        total = total_amount,
        "Guest order created"
    );

    let wants_card = matches!(body.payment_method, Some(PaymentMethod::Card));
    let (payment_url, resolved_method, checkout_failed) = if wants_card && state.sumup.is_some() {
        match provision_sumup_checkout_for_order(&state, order_token, &provisional_number, total_amount).await {
            Ok(url) => (Some(url), PaymentMethod::Card, false),
            Err(e) => {
                tracing::error!(error = %e, %order_token, "SumUp checkout creation failed — falling back to on-site");
                if let Err(e) = sqlx::query(
                    "UPDATE portal_guest_pending_order SET checkout_failed = true WHERE order_token = $1",
                )
                .bind(order_token)
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

    Ok(Json(GuestOrderResponse {
        order_token: order_token.to_string(),
        provisional_number,
        payment_method: resolved_method,
        total_amount,
        checkout_failed,
        guest_token: None,
        payment_url,
    }))
}

#[utoipa::path(
    get,
    path = "/guest/orders/{order_token}",
    tag = "Guest",
    params(
        ("order_token" = String, Path, description = "Order token UUID"),
    ),
    responses(
        (status = 200, description = "Guest order details", body = GuestOrderResponse),
        (status = 404, description = "Order not found"),
    )
)]
pub async fn get_guest_order(
    State(state): State<InvoiceState>,
    Path(order_token): Path<Uuid>,
) -> Result<Json<GuestOrderResponse>, AppError> {
    #[derive(FromRow)]
    struct OrderRow {
        provisional_number: String,
        payment_method: String,
        total_amount: f64,
        checkout_failed: bool,
        guest_token: Option<Uuid>,
    }

    let row = sqlx::query_as::<_, OrderRow>(
        r#"
        SELECT provisional_number, payment_method, total_amount,
               checkout_failed, guest_token
        FROM portal_guest_pending_order
        WHERE order_token = $1
        "#,
    )
    .bind(order_token)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(GuestOrderResponse {
        order_token: order_token.to_string(),
        provisional_number: row.provisional_number,
        payment_method: PaymentMethod::from(row.payment_method.as_str()),
        total_amount: row.total_amount,
        checkout_failed: row.checkout_failed,
        guest_token: row.guest_token.map(|t| t.to_string()),
        payment_url: None,
    }))
}

#[utoipa::path(
    get,
    path = "/guest/orders/{order_token}/payment-status",
    tag = "Guest",
    params(
        ("order_token" = String, Path, description = "Order token UUID"),
    ),
    responses(
        (status = 200, description = "Payment status for a guest order", body = GuestOrderPaymentStatusResponse),
        (status = 404, description = "Order not found"),
    )
)]
pub async fn get_guest_order_payment_status(
    State(state): State<InvoiceState>,
    Path(order_token): Path<Uuid>,
) -> Result<Json<GuestOrderPaymentStatusResponse>, AppError> {
    #[derive(FromRow)]
    struct OrderStatusRow {
        payment_method: String,
        sumup_checkout_id: Option<String>,
        paid_at: Option<chrono::DateTime<Utc>>,
        guest_token: Option<Uuid>,
        provisional_number: String,
    }

    let row = sqlx::query_as::<_, OrderStatusRow>(
        r#"
        SELECT payment_method, sumup_checkout_id, paid_at, guest_token, provisional_number
        FROM portal_guest_pending_order
        WHERE order_token = $1
        "#,
    )
    .bind(order_token)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // Bill already created
    if let Some(guest_token) = row.guest_token {
        return Ok(Json(GuestOrderPaymentStatusResponse {
            paid: true,
            guest_token: Some(guest_token.to_string()),
        }));
    }

    // Marked as paid but bill creation not yet complete (webhook + poll race)
    if row.paid_at.is_some() {
        return finalize_order_payment(&state, order_token).await;
    }

    // Not yet paid — check payment source
    let Some(sumup) = &state.sumup else {
        return Ok(Json(GuestOrderPaymentStatusResponse { paid: false, guest_token: None }));
    };

    let payment_method = PaymentMethod::from(row.payment_method.as_str());
    let payment_confirmed = match payment_method {
        PaymentMethod::Card => {
            if let Some(checkout_id) = &row.sumup_checkout_id {
                match sumup.get_checkout(checkout_id).await {
                    Ok(CheckoutStatus::Paid) => true,
                    Ok(_) => false,
                    Err(e) => {
                        tracing::warn!(error = %e, %order_token, "Failed to check SumUp checkout status");
                        false
                    }
                }
            } else {
                false
            }
        }
        PaymentMethod::OnSite => {
            let oldest_time = Utc::now() - chrono::Duration::minutes(10);
            match sumup.get_recent_transactions(oldest_time).await {
                Ok(txs) => txs.into_iter().any(|t| {
                    t.description
                        .as_deref()
                        .map(|d| d.trim().eq_ignore_ascii_case(&row.provisional_number))
                        .unwrap_or(false)
                }),
                Err(e) => {
                    tracing::warn!(error = %e, %order_token, "Failed to fetch recent SumUp transactions");
                    false
                }
            }
        }
    };

    if !payment_confirmed {
        return Ok(Json(GuestOrderPaymentStatusResponse { paid: false, guest_token: None }));
    }

    // Mark paid and create bill
    if let Err(e) = sqlx::query(
        "UPDATE portal_guest_pending_order SET paid_at = NOW() WHERE order_token = $1 AND paid_at IS NULL",
    )
    .bind(order_token)
    .execute(&state.db)
    .await
    {
        tracing::error!(error = %e, %order_token, "Failed to mark order as paid");
        return Ok(Json(GuestOrderPaymentStatusResponse { paid: false, guest_token: None }));
    }

    finalize_order_payment(&state, order_token).await
}

async fn finalize_order_payment(
    state: &InvoiceState,
    order_token: Uuid,
) -> Result<Json<GuestOrderPaymentStatusResponse>, AppError> {
    match create_bill_for_order(state, order_token).await {
        Ok(Some(gt)) => Ok(Json(GuestOrderPaymentStatusResponse {
            paid: true,
            guest_token: Some(gt.to_string()),
        })),
        Ok(None) => Ok(Json(GuestOrderPaymentStatusResponse { paid: true, guest_token: None })),
        Err(e) => {
            tracing::error!(error = %e, %order_token, "Bill creation failed during payment finalization");
            Ok(Json(GuestOrderPaymentStatusResponse { paid: true, guest_token: None }))
        }
    }
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

        let order_token: Option<Uuid> = sqlx::query_scalar(
            "SELECT order_token FROM portal_guest_pending_order WHERE sumup_checkout_id = $1",
        )
        .bind(&checkout_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        let Some(order_token) = order_token else {
            tracing::warn!(checkout_id, "SumUp webhook: no pending order found for checkout_id");
            return;
        };

        match status {
            CheckoutStatus::Paid => {
                if let Err(e) = sqlx::query(
                    "UPDATE portal_guest_pending_order SET paid_at = NOW() WHERE order_token = $1 AND paid_at IS NULL",
                )
                .bind(order_token)
                .execute(&state.db)
                .await
                {
                    tracing::error!(%order_token, error = %e, "SumUp webhook: failed to mark order as paid");
                    return;
                }
                tracing::info!(%order_token, checkout_id, "SumUp webhook: order marked as paid, creating bill");
                match create_bill_for_order(&state, order_token).await {
                    Ok(Some(gt)) => tracing::info!(%order_token, guest_token = %gt, "SumUp webhook: bill created"),
                    Ok(None) => tracing::warn!(%order_token, "SumUp webhook: bill creation returned None"),
                    Err(e) => tracing::error!(%order_token, error = %e, "SumUp webhook: bill creation failed"),
                }
            }
            CheckoutStatus::Failed => {
                tracing::info!(%order_token, checkout_id, "SumUp webhook: payment failed — deleting pending order");
                let _ = sqlx::query("DELETE FROM portal_guest_pending_order WHERE order_token = $1")
                    .bind(order_token)
                    .execute(&state.db)
                    .await;
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
    path = "/guest/pending-orders",
    tag = "Guest",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Pending on-site guest orders", body = PendingOrdersResponse),
        (status = 401, description = "Not authenticated"),
    )
)]
pub async fn list_pending_orders(
    _current_user: CurrentUser,
    State(state): State<InvoiceState>,
) -> Result<Json<PendingOrdersResponse>, AppError> {
    #[derive(FromRow)]
    struct Row {
        order_token: Uuid,
        provisional_number: String,
        total_amount: f64,
        created_at: chrono::DateTime<Utc>,
        guest_email: String,
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT order_token, provisional_number, total_amount, created_at, guest_email
        FROM portal_guest_pending_order
        WHERE payment_method = 'on_site'
          AND paid_at IS NULL
          AND expires_at > NOW()
        ORDER BY created_at ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(|r| PendingOrderItem {
            order_token: r.order_token.to_string(),
            provisional_number: r.provisional_number,
            total_amount: r.total_amount,
            created_at: r.created_at.to_rfc3339(),
            guest_email: r.guest_email,
        })
        .collect();

    Ok(Json(PendingOrdersResponse { data }))
}

#[utoipa::path(
    post,
    path = "/guest/orders/{order_token}/validate",
    tag = "Guest",
    security(("bearer_auth" = [])),
    params(
        ("order_token" = String, Path, description = "Order token UUID"),
    ),
    responses(
        (status = 200, description = "Order validated and bill created", body = ValidateOrderResponse),
        (status = 400, description = "Order already paid"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Order not found"),
    )
)]
pub async fn validate_guest_order(
    current_user: CurrentUser,
    State(state): State<InvoiceState>,
    Path(order_token): Path<Uuid>,
) -> Result<Json<ValidateOrderResponse>, AppError> {
    #[derive(FromRow)]
    struct OrderRow {
        provisional_number: String,
        paid_at: Option<chrono::DateTime<Utc>>,
    }

    let order = sqlx::query_as::<_, OrderRow>(
        "SELECT provisional_number, paid_at FROM portal_guest_pending_order WHERE order_token = $1",
    )
    .bind(order_token)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    if order.paid_at.is_some() {
        return Err(AppError::BadRequest("Order is already paid".into()));
    }

    sqlx::query(
        "UPDATE portal_guest_pending_order SET paid_at = NOW() WHERE order_token = $1 AND paid_at IS NULL",
    )
    .bind(order_token)
    .execute(&state.db)
    .await?;

    let guest_token = create_bill_for_order(&state, order_token)
        .await?
        .ok_or_else(|| AppError::External("Bill creation returned no token".into()))?;

    tracing::info!(
        %order_token,
        validator_user_id = current_user.id,
        provisional_number = &order.provisional_number,
        %guest_token,
        "Guest order validated manually by member"
    );

    Ok(Json(ValidateOrderResponse {
        guest_token: guest_token.to_string(),
    }))
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
    let bill_id: i32 = sqlx::query_scalar(
        "SELECT bill_id FROM portal_guest_bill WHERE guest_token = $1",
    )
    .bind(token)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    proxy_bill_pdf(&state, bill_id).await
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn provision_sumup_checkout_for_order(
    state: &InvoiceState,
    order_token: Uuid,
    provisional_number: &str,
    amount: f64,
) -> anyhow::Result<String> {
    let sumup = state
        .sumup
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("SumUp is not configured"))?;
    let redirect = format!("{}/buy/processing/{}", state.config.app_base_url, order_token);
    let webhook = format!("{}/api/guest/payment/webhook", state.config.app_base_url);
    let created = sumup
        .create_checkout(&order_token.to_string(), provisional_number, amount, &redirect, &webhook)
        .await?;
    if let Err(e) = sqlx::query(
        "UPDATE portal_guest_pending_order SET sumup_checkout_id = $1, payment_method = $2, checkout_failed = false WHERE order_token = $3",
    )
    .bind(&created.checkout_id)
    .bind(PaymentMethod::Card.as_str())
    .bind(order_token)
    .execute(&state.db)
    .await
    {
        tracing::error!(error = %e, "Failed to store SumUp checkout_id on pending order");
    }
    Ok(created.checkout_url)
}

/// Create a real bill and provision Unify vouchers for a paid pending order.
/// Protected by pg_advisory_xact_lock(42) — at most one bill is ever created per order.
/// Returns the guest_token if creation succeeded or was already done; None if order is not found/paid.
async fn create_bill_for_order(
    state: &InvoiceState,
    order_token: Uuid,
) -> Result<Option<Uuid>, AppError> {
    #[derive(FromRow)]
    struct PendingOrderRow {
        provisional_number: String,
        payment_method: String,
        guest_email: String,
        billing_address: String,
        total_amount: f64,
        guest_token: Option<Uuid>,
    }

    #[derive(FromRow)]
    struct OrderLineRow {
        service_id: i32,
        quantity: i32,
    }

    // Load order (must be paid)
    let Some(order) = sqlx::query_as::<_, PendingOrderRow>(
        r#"
        SELECT provisional_number, payment_method, guest_email, billing_address,
               total_amount, guest_token
        FROM portal_guest_pending_order
        WHERE order_token = $1 AND paid_at IS NOT NULL
        "#,
    )
    .bind(order_token)
    .fetch_optional(&state.db)
    .await? else {
        return Ok(None);
    };

    // Fast path: bill already created
    if let Some(gt) = order.guest_token {
        return Ok(Some(gt));
    }

    // Load lines
    let lines = sqlx::query_as::<_, OrderLineRow>(
        "SELECT service_id, quantity FROM portal_guest_pending_order_line WHERE order_token = $1",
    )
    .bind(order_token)
    .fetch_all(&state.db)
    .await?;

    // Load services
    let mut service_lines: Vec<(Service, i32)> = Vec::with_capacity(lines.len());
    for line in &lines {
        let service = repository::fetch_guest_service(&state.db, line.service_id).await?;
        service_lines.push((service, line.quantity));
    }

    let now = Utc::now();

    // Begin transaction with bill-creation lock (serializes all bill number generation)
    let mut tx = state.db.begin().await?;

    sqlx::query("SELECT pg_advisory_xact_lock(42)")
        .execute(&mut *tx)
        .await?;

    // Re-check inside lock: another thread may have committed while we waited
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT guest_token FROM portal_guest_pending_order WHERE order_token = $1",
    )
    .bind(order_token)
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    if let Some(gt) = existing {
        return Ok(Some(gt));
    }

    let guest_token = Uuid::new_v4();

    // Atomic claim: set guest_token now so concurrent callers skip on re-check.
    // If anything below fails, the transaction rolls back and guest_token reverts to NULL.
    let claimed: Option<Uuid> = sqlx::query_scalar(
        "UPDATE portal_guest_pending_order SET guest_token = $1 WHERE order_token = $2 AND guest_token IS NULL AND paid_at IS NOT NULL RETURNING order_token",
    )
    .bind(guest_token)
    .bind(order_token)
    .fetch_optional(&mut *tx)
    .await?;

    if claimed.is_none() {
        return Ok(None);
    }

    // Compute next bill number
    let last_number: Option<String> =
        sqlx::query_scalar("SELECT number FROM billjobs_bill ORDER BY id DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;
    let number = next_bill_number(last_number.as_deref(), now.date_naive());

    tracing::info!(
        %order_token,
        bill_number = &number,
        provisional_number = &order.provisional_number,
        "Creating bill for paid order"
    );

    // Insert bill (isPaid = true because payment was already confirmed)
    let bill_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO billjobs_bill
            (number, user_id, billing_date, amount, issuer_address, billing_address, "isPaid")
        VALUES ($1, $2, $3, $4, $5, $6, true)
        RETURNING id
        "#,
    )
    .bind(&number)
    .bind(state.config.guest_user_id)
    .bind(now.date_naive())
    .bind(order.total_amount)
    .bind(&state.config.issuer_address)
    .bind(&order.billing_address)
    .fetch_one(&mut *tx)
    .await?;

    // Link to guest token
    sqlx::query(
        "INSERT INTO portal_guest_bill (guest_token, bill_id, payment_method, guest_email) VALUES ($1, $2, $3, $4)",
    )
    .bind(guest_token)
    .bind(bill_id)
    .bind(&order.payment_method)
    .bind(&order.guest_email)
    .execute(&mut *tx)
    .await?;

    // For each line: insert billline, provision Unify vouchers, persist vouchers
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

    tx.commit().await?;

    // Send confirmation email (non-blocking — failure does not fail the caller)
    if let Some(smtp) = state.smtp.clone() {
        let to = order.guest_email.clone();
        let summary_url = format!("{}/buy/summary/{}", state.config.app_base_url, guest_token);
        let body_text = build_confirmation_body(&number, &now.date_naive().to_string(), order.total_amount, &response_lines, &summary_url);
        tokio::spawn(async move {
            if let Err(e) = crate::users::email::send_smtp_email(&smtp, &to, "Commande confirmée", body_text).await {
                tracing::warn!(error = %e, "Failed to send guest order confirmation email");
            }
        });
    }

    Ok(Some(guest_token))
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
