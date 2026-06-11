use sqlx::FromRow;

use crate::error::AppError;

use super::domain::{Service, VoucherSpec};

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

fn row_to_service(row: ServiceRow) -> Result<Service, AppError> {
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

/// Fetch a single available service by id.
pub async fn fetch_service(db: &sqlx::PgPool, id: i32) -> Result<Service, AppError> {
    let row = sqlx::query_as::<_, ServiceRow>(
        r#"
        SELECT id, name, description, price, kind, amount, duration, external_service_id
        FROM portal_service
        WHERE id = $1 AND is_available = true
        "#,
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    row_to_service(row)
}

/// Fetch a single available, guest-available service by id.
pub async fn fetch_guest_service(db: &sqlx::PgPool, id: i32) -> Result<Service, AppError> {
    let row = sqlx::query_as::<_, ServiceRow>(
        r#"
        SELECT id, name, description, price, kind, amount, duration, external_service_id
        FROM portal_service
        WHERE id = $1 AND is_available = true AND is_guest_available = true
        "#,
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    row_to_service(row)
}

/// List all available services.
pub async fn list_available_services(db: &sqlx::PgPool) -> Result<Vec<Service>, AppError> {
    let rows = sqlx::query_as::<_, ServiceRow>(
        r#"
        SELECT id, name, description, price, kind, amount, duration, external_service_id
        FROM portal_service
        WHERE is_available = true
        ORDER BY id
        "#,
    )
    .fetch_all(db)
    .await?;

    rows.into_iter().map(row_to_service).collect()
}

/// List all available, guest-available services.
pub async fn list_guest_available_services(db: &sqlx::PgPool) -> Result<Vec<Service>, AppError> {
    let rows = sqlx::query_as::<_, ServiceRow>(
        r#"
        SELECT id, name, description, price, kind, amount, duration, external_service_id
        FROM portal_service
        WHERE is_available = true AND is_guest_available = true
        ORDER BY id
        "#,
    )
    .fetch_all(db)
    .await?;

    rows.into_iter().map(row_to_service).collect()
}
