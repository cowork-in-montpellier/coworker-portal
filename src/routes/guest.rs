use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::header,
    response::Response,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState,
    auth::routes::acquire_django_session,
    domain::{Service, VoucherSpec, VoucherStatus, format_code, next_bill_number, resolve_voucher_params},
    error::AppError,
    unify::CreateVouchersRequest,
};

// ─── Service row ──────────────────────────────────────────────────────────────

#[derive(FromRow)]
struct GuestServiceRow {
    id: i32,
    name: String,
    description: String,
    price: f64,
    kind: String,
    amount: Option<i32>,
    duration: Option<i32>,
    external_service_id: i32,
}

impl TryFrom<GuestServiceRow> for Service {
    type Error = AppError;

    fn try_from(row: GuestServiceRow) -> Result<Self, Self::Error> {
        let voucher_spec = match row.kind.as_str() {
            "Monthly" => VoucherSpec::Monthly,
            "Book" => VoucherSpec::Book {
                amount: row.amount.unwrap_or(1),
                duration: row.duration.unwrap_or(1),
            },
            _ => return Err(AppError::NotFound),
        };
        Ok(Service {
            id: row.id,
            name: row.name,
            description: row.description,
            price: row.price,
            voucher_spec,
            external_service_id: row.external_service_id,
        })
    }
}

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
    pub lines: Vec<GuestBillLineResponse>,
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
    /// Optional customer name — prepended to billing_address so it appears in the Django-generated PDF.
    pub billing_name: Option<String>,
    /// Optional billing address lines.
    pub billing_address: Option<String>,
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
    State(state): State<AppState>,
) -> Result<Json<GuestServicesResponse>, AppError> {
    let rows = sqlx::query_as::<_, GuestServiceRow>(
        r#"
        SELECT id, name, description, price, kind, amount, duration, external_service_id
        FROM portal_service
        WHERE is_available = true AND is_guest_available = true
        ORDER BY id
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let data = rows
        .into_iter()
        .map(Service::try_from)
        .collect::<Result<Vec<_>, _>>()?;

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
    State(state): State<AppState>,
    Json(body): Json<CreateGuestBillRequest>,
) -> Result<Json<GuestBillResponse>, AppError> {
    if body.lines.is_empty() {
        return Err(crate::error::AppError::BadRequest("At least one line is required".into()));
    }

    let now = Utc::now();

    // 1. Fetch all services (must all be guest-available); validate quantity >= 1
    let mut service_lines: Vec<(Service, i32)> = Vec::with_capacity(body.lines.len());
    for line_req in &body.lines {
        if line_req.quantity < 1 {
            return Err(AppError::BadRequest(format!("Quantity must be at least 1 (got {})", line_req.quantity)));
        }
        let service_row = sqlx::query_as::<_, GuestServiceRow>(
            r#"
            SELECT id, name, description, price, kind, amount, duration, external_service_id
            FROM portal_service
            WHERE id = $1 AND is_available = true AND is_guest_available = true
            "#,
        )
        .bind(line_req.service_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
        service_lines.push((Service::try_from(service_row)?, line_req.quantity));
    }

    // 2. Compute total amount (price × quantity per line)
    let total_amount: f64 = service_lines.iter().map(|(s, q)| s.price * (*q as f64)).sum();

    // 3. Build billing address — name prepended if provided
    let fallback_address: String = sqlx::query_scalar(
        "SELECT billing_address FROM billjobs_userprofile WHERE user_id = $1",
    )
    .bind(state.config.guest_user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or_default();

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

    // 8. Link bill to guest token
    sqlx::query("INSERT INTO portal_guest_bill (guest_token, bill_id) VALUES ($1, $2)")
        .bind(guest_token)
        .bind(bill_id)
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
            .map_err(|e| AppError::Unify(e.to_string()))?;

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

    Ok(Json(GuestBillResponse {
        guest_token: guest_token.to_string(),
        bill_id,
        bill_number: number,
        date: now.date_naive().to_string(),
        amount: total_amount,
        is_paid: false,
        lines: response_lines,
    }))
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
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Result<Json<GuestBillResponse>, AppError> {
    #[derive(FromRow)]
    struct GuestBillRow {
        id: i32,
        number: String,
        billing_date: chrono::NaiveDate,
        amount: f64,
        is_paid: bool,
    }

    let row = sqlx::query_as::<_, GuestBillRow>(
        r#"
        SELECT b.id, b.number, b.billing_date, b.amount, b."isPaid" AS is_paid
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
        lines,
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
        (status = 200, description = "Live voucher status from Unify", body = crate::routes::vouchers::VoucherCheckResponse),
        (status = 404, description = "Bill not found"),
    )
)]
pub async fn check_guest_vouchers(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Result<Json<crate::routes::vouchers::VoucherCheckResponse>, AppError> {
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
        .map_err(|e| AppError::Unify(e.to_string()))?;

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

        data.push(crate::routes::vouchers::VoucherStatusResponse {
            unify_id: r.unify_id,
            code: format_code(&r.code),
            duration: r.duration,
            status: status.as_str().to_string(),
        });
    }

    Ok(Json(crate::routes::vouchers::VoucherCheckResponse { data }))
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
    State(state): State<AppState>,
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

/// Proxy a Django invoice PDF using the cached superuser session.
/// Shared by both authenticated (`bill_pdf`) and guest (`guest_bill_pdf`) routes.
pub async fn proxy_bill_pdf(state: &AppState, bill_id: i32) -> Result<Response, AppError> {
    let session = {
        let guard = state.superuser_session.read().await;
        guard.clone()
    };

    let session = session.ok_or_else(|| {
        AppError::Unify("No superuser Django session available — configure DJANGO_SUPERUSER_USERNAME/PASSWORD".into())
    })?;

    match fetch_django_pdf(state, bill_id, &session).await {
        Ok(response) => Ok(response),
        Err(_) => {
            // Session may have expired — try to re-acquire once
            tracing::warn!("Guest PDF: Django returned error, attempting session refresh");
            let new_session = acquire_django_session(
                &state.config.django_base_url,
                state.config.django_accept_invalid_certs,
                &state.config.django_superuser_username,
                &state.config.django_superuser_password,
            )
            .await
            .map_err(|e| AppError::Unify(format!("Session refresh failed: {e}")))?;

            {
                let mut guard = state.superuser_session.write().await;
                *guard = Some(new_session.clone());
            }

            fetch_django_pdf(state, bill_id, &new_session).await
        }
    }
}

async fn fetch_django_pdf(state: &AppState, bill_id: i32, session: &str) -> Result<Response, AppError> {
    let url = format!(
        "{}/billjobs/generate_pdf/{}",
        state.config.django_base_url, bill_id
    );

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(state.config.django_accept_invalid_certs)
        .build()
        .map_err(|e| AppError::Unify(e.to_string()))?;

    let res = client
        .get(&url)
        .header("Cookie", format!("sessionid={}", session))
        .send()
        .await
        .map_err(|e| AppError::Unify(e.to_string()))?;

    if !res.status().is_success() {
        return Err(AppError::Unify(format!("Django returned {}", res.status())));
    }

    let content_disposition = res.headers().get(header::CONTENT_DISPOSITION).cloned();
    let bytes = res.bytes().await.map_err(|e| AppError::Unify(e.to_string()))?;

    let mut builder = Response::builder().header(header::CONTENT_TYPE, "application/pdf");
    if let Some(cd) = content_disposition {
        builder = builder.header(header::CONTENT_DISPOSITION, cd);
    }

    Ok(builder.body(Body::from(bytes)).unwrap())
}
