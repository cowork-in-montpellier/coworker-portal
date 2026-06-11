use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::invoice::domain::Service;
use crate::invoice::repository;
use crate::invoice::state::State as InvoiceState;
use crate::users::auth::CurrentUser;

#[derive(Serialize, ToSchema)]
pub struct ServicesResponse {
    pub data: Vec<Service>,
}

#[utoipa::path(
    get,
    path = "/services",
    tag = "Services",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of available services", body = ServicesResponse),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_services(
    State(state): State<InvoiceState>,
    _user: CurrentUser,
) -> Result<Json<ServicesResponse>, AppError> {
    let data = repository::list_available_services(&state.db).await?;
    Ok(Json(ServicesResponse { data }))
}
