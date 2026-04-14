use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

use crate::{
    AppState,
    auth::CurrentUser,
    domain::{
        Bill, Service, VoucherSpec, Voucher, VoucherStatus,
        format_code, next_bill_number, resolve_voucher_params,
    },
    error::AppError,
    unify::CreateVouchersRequest,
};

// ─── Request / Response types ────────────────────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct CreateBillRequest {
    pub service_id: i32,
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

/// A bill that references a service managed by this application.
#[derive(Serialize, ToSchema)]
pub struct ManagedBill {
    pub id: i32,
    pub number: String,
    pub date: NaiveDate,
    pub amount: f64,
    pub is_paid: bool,
    pub service_id: i32,
    pub vouchers: Vec<VoucherResponse>,
}

/// A bill that exists in the external system but references a service
/// not known to this application (created externally or via a removed service).
#[derive(Serialize, ToSchema)]
pub struct UnmanagedBill {
    pub id: i32,
    pub number: String,
    pub date: NaiveDate,
    pub amount: f64,
    pub is_paid: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(tag = "kind")]
pub enum BillResponse {
    Managed(ManagedBill),
    Unmanaged(UnmanagedBill),
}

#[derive(Serialize, ToSchema)]
pub struct VoucherResponse {
    pub unify_id: String,
    pub code: String,
    pub duration: i32,
    pub status: String,
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
    service_id: Option<i32>, // our internal service.id; NULL when bill references an unmanaged service
}

#[derive(FromRow)]
struct VoucherRow {
    unify_id: String,
    bill_id: i32,
    unify_create_time: i64,
    code: String,
    created_at: chrono::DateTime<Utc>,
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

fn row_to_voucher(row: VoucherRow) -> Voucher {
    Voucher {
        unify_id: row.unify_id,
        bill_id: row.bill_id,
        unify_create_time: row.unify_create_time,
        code: row.code,
        created_at: row.created_at,
        duration: row.duration,
        status: VoucherStatus::from(row.status.as_str()),
    }
}

fn to_bill_response(bill: &Bill) -> BillResponse {
    BillResponse::Managed(ManagedBill {
        id: bill.id,
        number: bill.number.clone(),
        date: bill.date,
        amount: bill.amount,
        is_paid: bill.is_paid,
        service_id: bill.service_id,
        vouchers: bill.vouchers.iter().map(|v| VoucherResponse {
            unify_id: v.unify_id.clone(),
            code: format_code(&v.code),
            duration: v.duration,
            status: v.status.as_str().to_string(),
        }).collect(),
    })
}

async fn row_to_bill_response(db: &sqlx::PgPool, row: BillRow) -> Result<BillResponse, AppError> {
    match row.service_id {
        Some(service_id) => {
            let vouchers = fetch_vouchers_for_bill(db, row.id).await?;
            Ok(BillResponse::Managed(ManagedBill {
                id: row.id,
                number: row.number,
                date: row.billing_date,
                amount: row.amount,
                is_paid: row.is_paid,
                service_id,
                vouchers: vouchers.iter().map(|v| VoucherResponse {
                    unify_id: v.unify_id.clone(),
                    code: format_code(&v.code),
                    duration: v.duration,
                    status: v.status.as_str().to_string(),
                }).collect(),
            }))
        }
        None => Ok(BillResponse::Unmanaged(UnmanagedBill {
            id: row.id,
            number: row.number,
            date: row.billing_date,
            amount: row.amount,
            is_paid: row.is_paid,
        })),
    }
}

async fn fetch_vouchers_for_bill(db: &sqlx::PgPool, bill_id: i32) -> Result<Vec<Voucher>, AppError> {
    let rows = sqlx::query_as::<_, VoucherRow>(
        "SELECT unify_id, bill_id, unify_create_time, code, created_at, duration, status FROM voucher WHERE bill_id = $1"
    )
    .bind(bill_id)
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(row_to_voucher).collect())
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
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Service not found or unavailable"),
    )
)]
pub async fn create_bill(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(body): Json<CreateBillRequest>,
) -> Result<Json<BillResponse>, AppError> {
    let now = Utc::now();

    // 1. Fetch service
    let service_row = sqlx::query_as::<_, ServiceRow>(
        r#"
        SELECT id, name, description, price, kind, amount, duration, external_service_id
        FROM service
        WHERE id = $1 AND is_available = true
        "#,
    )
    .bind(body.service_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let service = row_to_service(service_row)?;

    // 2. Compute voucher params
    let (voucher_count, duration_hours) = resolve_voucher_params(&service.voucher_spec, now);

    // 3. Begin transaction
    let mut tx = state.db.begin().await?;

    // 4. Acquire advisory lock then read last number — serializes concurrent bill creation.
    //    pg_advisory_xact_lock is released automatically on transaction commit/rollback.
    sqlx::query("SELECT pg_advisory_xact_lock(42)")
        .execute(&mut *tx)
        .await?;
    let last_number: Option<String> =
        sqlx::query_scalar("SELECT number FROM billjobs_bill ORDER BY id DESC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await?;
    let number = next_bill_number(last_number.as_deref(), now.date_naive());

    // 5. Fetch billing address from user profile
    let billing_address: String = sqlx::query_scalar(
        "SELECT billing_address FROM billjobs_userprofile WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or_default();

    tracing::info!(number= &number, "Create bills");

    // 6. Insert bill into billjobs_bill
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
    .bind(service.price)
    .bind(&state.config.issuer_address)
    .bind(&billing_address)
    .fetch_one(&mut *tx)
    .await?;

    tracing::info!(number= &number, "Create bills line");

    // 7. Insert bill line (persistence detail, quantity=1)
    sqlx::query(
        "INSERT INTO billjobs_billline (bill_id, service_id, quantity, total, note) VALUES ($1, $2, 1, $3, '')",
    )
    .bind(bill_id)
    .bind(service.external_service_id)
    .bind(service.price)
    .execute(&mut *tx)
    .await?;

    // 8. Provision vouchers on Unify — if this fails, tx is rolled back on drop
    let note = format!("{}_{}", number, user.first_name);
    let unify_vouchers = state
        .unify
        .create_vouchers(CreateVouchersRequest {
            n: voucher_count,
            duration_hours,
            note,
            quota: 2,
        })
        .await
        .map_err(|e| AppError::Unify(e.to_string()))?;

    // 9. Persist vouchers
    let mut vouchers = Vec::new();
    for uv in &unify_vouchers {
        sqlx::query(
            "INSERT INTO voucher (unify_id, bill_id, unify_create_time, code, created_at, duration, status) VALUES ($1, $2, $3, $4, $5, $6, 'Valid')",
        )
        .bind(&uv.unify_id)
        .bind(bill_id)
        .bind(uv.create_time)
        .bind(&uv.code)
        .bind(now)
        .bind(uv.duration)
        .execute(&mut *tx)
        .await?;

        vouchers.push(Voucher {
            unify_id: uv.unify_id.clone(),
            bill_id,
            unify_create_time: uv.create_time,
            code: uv.code.clone(),
            created_at: now,
            duration: uv.duration,
            status: VoucherStatus::Valid,
        });
    }

    // 10. Commit
    tx.commit().await?;

    let bill = Bill {
        id: bill_id,
        number,
        user_id: user.id,
        service_id: service.id,
        date: now.date_naive(),
        amount: service.price,
        is_paid: false,
        issuer_address: state.config.issuer_address.clone(),
        billing_address,
        vouchers,
    };

    Ok(Json(to_bill_response(&bill)))
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

    let rows = sqlx::query_as::<_, BillRow>(
        r#"
        SELECT b.id, b.number, b.billing_date, b.amount, b."isPaid" AS is_paid,
               (SELECT s.id FROM billjobs_billline bl
                JOIN service s ON s.external_service_id = bl.service_id
                WHERE bl.bill_id = b.id LIMIT 1) AS service_id
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

    let mut data = Vec::new();
    for row in rows {
        data.push(row_to_bill_response(&state.db, row).await?);
    }

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
    let row = sqlx::query_as::<_, BillRow>(
        r#"
        SELECT b.id, b.number, b.billing_date, b.amount, b."isPaid" AS is_paid,
               (SELECT s.id FROM billjobs_billline bl
                JOIN service s ON s.external_service_id = bl.service_id
                WHERE bl.bill_id = b.id LIMIT 1) AS service_id
        FROM billjobs_bill b
        WHERE b.id = $1 AND b.user_id = $2
        "#,
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(row_to_bill_response(&state.db, row).await?))
}
