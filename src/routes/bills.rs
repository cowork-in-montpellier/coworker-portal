use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};

use crate::{
    AppState,
    auth::CurrentUser,
    domain::{Service, VoucherSpec, format_code, next_bill_number, resolve_voucher_params},
    error::AppError,
    unify::CreateVouchersRequest,
};

// ─── Request / Response types ────────────────────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct CreateBillLineRequest {
    pub service_id: i32,
    #[serde(default = "default_quantity")]
    pub quantity: i32,
}
fn default_quantity() -> i32 { 1 }

#[derive(Deserialize, ToSchema)]
pub struct CreateBillRequest {
    pub lines: Vec<CreateBillLineRequest>,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListBillsQuery {
    /// Pagination offset (default: 0)
    #[serde(default = "default_offset")]
    #[param(required = false, minimum = 0)]
    pub offset: i64,
    /// Page size (default: 20)
    #[serde(default = "default_limit")]
    #[param(required = false, minimum = 1)]
    pub limit: i64,
    #[param(required = false, format = "date")]
    pub date_from: Option<NaiveDate>,
    #[param(required = false, format = "date")]
    pub date_to: Option<NaiveDate>,
    #[param(required = false)]
    pub number: Option<String>,
}
fn default_offset() -> i64 { 0 }
fn default_limit() -> i64 { 20 }

#[derive(Serialize, ToSchema)]
pub struct VoucherResponse {
    pub unify_id: String,
    pub code: String,
    pub duration: i32,
    pub status: String,
}

/// One line of a bill. `service_id` is None when the line references a service
/// not known to this application (created externally or via a removed service).
#[derive(Serialize, ToSchema)]
pub struct BillLineResponse {
    pub id: i32,
    pub service_id: Option<i32>,
    pub quantity: i32,
    pub vouchers: Vec<VoucherResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct BillResponse {
    pub id: i32,
    pub number: String,
    pub date: NaiveDate,
    pub amount: f64,
    pub is_paid: bool,
    pub lines: Vec<BillLineResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct ListBillsResponse {
    pub total: i64,
    pub data: Vec<BillResponse>,
}

// ─── DB row types ─────────────────────────────────────────────────────────────

#[derive(FromRow)]
struct ServiceRow {
    id: i32,
    name: String,
    description: String,
    price: f64,
    kind: String,
    amount: Option<i32>,
    duration: Option<i32>,
    external_service_id: i32,
}

#[derive(FromRow)]
struct BillRow {
    id: i32,
    number: String,
    billing_date: NaiveDate,
    amount: f64,
    is_paid: bool,
}

#[derive(FromRow)]
struct BillLineRow {
    line_id: i32,
    bill_id: i32,
    service_id: Option<i32>,
    quantity: i32,
    voucher_kind: Option<String>,
    voucher_amount: Option<i32>,
}

#[derive(FromRow, Clone)]
struct VoucherRow {
    unify_id: String,
    billline_id: i32,
    code: String,
    duration: i32,
    status: String,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn row_to_service(row: ServiceRow) -> Result<Service, AppError> {
    let voucher_spec = match row.kind.as_str() {
        "Monthly" => VoucherSpec::Monthly,
        "Book" => VoucherSpec::Book {
            amount: row.amount.unwrap_or(1),
            duration: row.duration.unwrap_or(1),
        },
        _ => return Err(AppError::NotFound),
    };
    Ok(Service { id: row.id, name: row.name, description: row.description, price: row.price, voucher_spec, external_service_id: row.external_service_id })
}

fn voucher_row_to_response(v: VoucherRow) -> VoucherResponse {
    VoucherResponse {
        unify_id: v.unify_id,
        code: format_code(&v.code),
        duration: v.duration,
        status: v.status,
    }
}

/// Bulk-fetch vouchers for a set of bill IDs, keyed by billline_id.
async fn fetch_vouchers_bulk(
    db: &sqlx::PgPool,
    bill_ids: &[i32],
) -> Result<HashMap<i32, Vec<VoucherRow>>, AppError> {
    if bill_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, VoucherRow>(
        "SELECT unify_id, billline_id, code, duration, status FROM portal_voucher WHERE bill_id = ANY($1)",
    )
    .bind(bill_ids)
    .fetch_all(db)
    .await?;

    let mut map: HashMap<i32, Vec<VoucherRow>> = HashMap::new();
    for row in rows {
        map.entry(row.billline_id).or_default().push(row);
    }
    Ok(map)
}

/// Bulk-fetch bill lines for a set of bill IDs, keyed by bill_id.
async fn fetch_lines_bulk(
    db: &sqlx::PgPool,
    bill_ids: &[i32],
) -> Result<HashMap<i32, Vec<BillLineRow>>, AppError> {
    if bill_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, BillLineRow>(
        r#"
        SELECT bl.id AS line_id, bl.bill_id, bl.quantity::int4 AS quantity,
               s.id AS service_id, s.kind AS voucher_kind, s.amount AS voucher_amount
        FROM billjobs_billline bl
        LEFT JOIN portal_service s ON s.external_service_id = bl.service_id
        WHERE bl.bill_id = ANY($1)
        "#,
    )
    .bind(bill_ids)
    .fetch_all(db)
    .await?;

    let mut map: HashMap<i32, Vec<BillLineRow>> = HashMap::new();
    for row in rows {
        map.entry(row.bill_id).or_default().push(row);
    }
    Ok(map)
}

fn expected_voucher_count(row: &BillLineRow) -> usize {
    match row.voucher_kind.as_deref() {
        Some("Book") => row.voucher_amount.unwrap_or(0) as usize * row.quantity as usize,
        Some("Monthly") => row.quantity as usize,
        _ => 0,
    }
}

/// When a billjobs_service maps to multiple portal_service rows the LEFT JOIN
/// produces one BillLineRow per match. Pick the candidate whose expected voucher
/// count matches the actual count; fall back to the first candidate when there
/// are no vouchers yet (e.g. line just inserted).
fn deduplicate_lines(
    lines: Vec<BillLineRow>,
    vouchers_by_line: &HashMap<i32, Vec<VoucherRow>>,
) -> Vec<BillLineRow> {
    let mut order: Vec<i32> = Vec::new();
    let mut groups: HashMap<i32, Vec<BillLineRow>> = HashMap::new();
    for row in lines {
        if !groups.contains_key(&row.line_id) {
            order.push(row.line_id);
        }
        groups.entry(row.line_id).or_default().push(row);
    }
    order.into_iter().map(|line_id| {
        let mut candidates = groups.remove(&line_id).unwrap();
        if candidates.len() == 1 {
            return candidates.remove(0);
        }
        let actual = vouchers_by_line.get(&line_id).map(|v| v.len()).unwrap_or(0);
        if actual == 0 {
            return candidates.remove(0);
        }
        let pos = candidates.iter().position(|c| expected_voucher_count(c) == actual);
        candidates.remove(pos.unwrap_or(0))
    }).collect()
}

fn assemble_bill(
    bill: BillRow,
    lines: Vec<BillLineRow>,
    vouchers_by_line: &mut HashMap<i32, Vec<VoucherRow>>,
) -> BillResponse {
    let lines = deduplicate_lines(lines, vouchers_by_line);
    let bill_lines = lines.into_iter().map(|l| {
        let vouchers = vouchers_by_line
            .get(&l.line_id)
            .cloned()
            .unwrap_or_else(|| vec![])
            .into_iter()
            .map(voucher_row_to_response)
            .collect();
        BillLineResponse { id: l.line_id, service_id: l.service_id, quantity: l.quantity, vouchers }
    }).collect();

    BillResponse {
        id: bill.id,
        number: bill.number,
        date: bill.billing_date,
        amount: bill.amount,
        is_paid: bill.is_paid,
        lines: bill_lines,
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/bills",
    tag = "Bills",
    security(("bearer_auth" = [])),
    request_body = CreateBillRequest,
    responses(
        (status = 200, description = "Bill created with vouchers", body = BillResponse),
        (status = 400, description = "No lines provided"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Service not found or unavailable"),
    )
)]
pub async fn create_bill(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CreateBillRequest>,
) -> Result<Json<BillResponse>, AppError> {
    if body.lines.is_empty() {
        return Err(AppError::BadRequest("At least one line is required".into()));
    }

    let now = Utc::now();

    // 1. Fetch and validate all requested services upfront; also validate quantity >= 1
    let mut service_lines: Vec<(Service, i32)> = Vec::with_capacity(body.lines.len());
    for line_req in &body.lines {
        if line_req.quantity < 1 {
            return Err(AppError::BadRequest(format!("Quantity must be at least 1 (got {})", line_req.quantity)));
        }
        let service_row = sqlx::query_as::<_, ServiceRow>(
            r#"
            SELECT id, name, description, price, kind, amount, duration, external_service_id
            FROM portal_service
            WHERE id = $1 AND is_available = true
            "#,
        )
        .bind(line_req.service_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
        service_lines.push((row_to_service(service_row)?, line_req.quantity));
    }

    // 2. Compute total bill amount (price × quantity per line)
    let total_amount: f64 = service_lines.iter().map(|(s, q)| s.price * (*q as f64)).sum();

    // 3. Begin transaction
    let mut tx = state.db.begin().await?;

    // 4. Acquire advisory lock + compute next bill number
    sqlx::query("SELECT pg_advisory_xact_lock(42)")
        .execute(&mut *tx)
        .await?;
    let last_number: Option<String> =
        sqlx::query_scalar("SELECT number FROM billjobs_bill ORDER BY id DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;
    let number = next_bill_number(last_number.as_deref(), now.date_naive());

    // 5. Fetch billing address
    let billing_address: String = sqlx::query_scalar(
        "SELECT billing_address FROM billjobs_userprofile WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or_default();

    tracing::info!(number = &number, lines = service_lines.len(), "Creating bill");

    // 6. Insert bill
    let bill_id: i32 = sqlx::query_scalar(
        r#"
        INSERT INTO billjobs_bill
            (number, user_id, billing_date, amount, issuer_address, billing_address, "isPaid")
        VALUES ($1, $2, $3, $4, $5, $6, false)
        RETURNING id
        "#,
    )
    .bind(&number)
    .bind(user.id)
    .bind(now.date_naive())
    .bind(total_amount)
    .bind(&state.config.issuer_address)
    .bind(&billing_address)
    .fetch_one(&mut *tx)
    .await?;

    // 7. For each line: insert bill line, provision vouchers, persist vouchers
    let mut response_lines: Vec<BillLineResponse> = Vec::with_capacity(service_lines.len());

    for (service, quantity) in &service_lines {
        let (voucher_count, duration_hours) = resolve_voucher_params(&service.voucher_spec, now);
        let line_total = service.price * (*quantity as f64);
        let total_vouchers = voucher_count * quantity;

        tracing::info!(service=service.name, quantity=&quantity, "Creating voucher for {number}");

        let billline_id: i32 = sqlx::query_scalar(
            "INSERT INTO billjobs_billline (bill_id, service_id, quantity, total, note) VALUES ($1, $2, $3, $4, '') RETURNING id",
        )
        .bind(bill_id)
        .bind(service.external_service_id)
        .bind(*quantity as i16)
        .bind(line_total)
        .fetch_one(&mut *tx)
        .await?;

        let note = format!("{}_{}_{}", number, service.external_service_id, user.first_name);

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

        let mut line_vouchers: Vec<VoucherResponse> = Vec::with_capacity(unify_vouchers.len());
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

            line_vouchers.push(VoucherResponse {
                unify_id: uv.unify_id.clone(),
                code: format_code(&uv.code),
                duration: uv.duration,
                status: "Valid".to_string(),
            });
        }

        response_lines.push(BillLineResponse {
            id: billline_id,
            service_id: Some(service.id),
            quantity: *quantity,
            vouchers: line_vouchers,
        });
    }

    // 8. Commit
    tx.commit().await?;

    Ok(Json(BillResponse {
        id: bill_id,
        number,
        date: now.date_naive(),
        amount: total_amount,
        is_paid: false,
        lines: response_lines,
    }))
}

#[utoipa::path(
    get,
    path = "/bills",
    tag = "Bills",
    security(("bearer_auth" = [])),
    params(ListBillsQuery),
    responses(
        (status = 200, description = "Paginated list of bills for the authenticated user", body = ListBillsResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_bills(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(q): Query<ListBillsQuery>,
) -> Result<Json<ListBillsResponse>, AppError> {
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM billjobs_bill
        WHERE user_id = $1
          AND ($2::text   IS NULL OR number       = $2)
          AND ($3::date   IS NULL OR billing_date >= $3)
          AND ($4::date   IS NULL OR billing_date <= $4)
        "#,
    )
    .bind(user.id)
    .bind(&q.number)
    .bind(q.date_from)
    .bind(q.date_to)
    .fetch_one(&state.db)
    .await?;

    // Query 1: paginated bills
    let bill_rows = sqlx::query_as::<_, BillRow>(
        r#"
        SELECT b.id, b.number, b.billing_date, b.amount, b."isPaid" AS is_paid
        FROM billjobs_bill b
        WHERE b.user_id = $1
          AND ($2::text   IS NULL OR b.number       = $2)
          AND ($3::date   IS NULL OR b.billing_date >= $3)
          AND ($4::date   IS NULL OR b.billing_date <= $4)
        ORDER BY b.id DESC
        LIMIT $5 OFFSET $6
        "#,
    )
    .bind(user.id)
    .bind(&q.number)
    .bind(q.date_from)
    .bind(q.date_to)
    .bind(q.limit)
    .bind(q.offset)
    .fetch_all(&state.db)
    .await?;

    let bill_ids: Vec<i32> = bill_rows.iter().map(|r| r.id).collect();

    // Query 2 + 3: bill lines and vouchers for all fetched bills
    let mut lines_by_bill = fetch_lines_bulk(&state.db, &bill_ids).await?;
    let mut vouchers_by_line = fetch_vouchers_bulk(&state.db, &bill_ids).await?;

    let data = bill_rows.into_iter().map(|bill| {
        let lines = lines_by_bill.remove(&bill.id).unwrap_or_default();
        assemble_bill(bill, lines, &mut vouchers_by_line)
    }).collect();

    Ok(Json(ListBillsResponse { total, data }))
}

#[utoipa::path(
    get,
    path = "/bills/{id}",
    tag = "Bills",
    security(("bearer_auth" = [])),
    params(
        ("id" = i32, Path, description = "Bill ID"),
    ),
    responses(
        (status = 200, description = "Bill details", body = BillResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Bill not found"),
    )
)]
pub async fn get_bill(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i32>,
) -> Result<Json<BillResponse>, AppError> {
    let bill_row = sqlx::query_as::<_, BillRow>(
        r#"
        SELECT b.id, b.number, b.billing_date, b.amount, b."isPaid" AS is_paid
        FROM billjobs_bill b
        WHERE b.id = $1 AND b.user_id = $2
        "#,
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let mut lines_by_bill = fetch_lines_bulk(&state.db, &[bill_row.id]).await?;
    let mut vouchers_by_line = fetch_vouchers_bulk(&state.db, &[bill_row.id]).await?;
    let lines = lines_by_bill.remove(&bill_row.id).unwrap_or_default();

    Ok(Json(assemble_bill(bill_row, lines, &mut vouchers_by_line)))
}
