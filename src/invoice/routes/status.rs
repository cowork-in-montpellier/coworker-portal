use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::invoice::state::State as InvoiceState;

#[derive(Serialize, ToSchema)]
pub struct StatusResponse {
    /// Whether invoice PDF download is available (superuser Django session is active).
    pub invoice_available: bool,
    /// Whether SumUp online payment is configured and available.
    pub sumup_available: bool,
}

#[utoipa::path(
    get,
    path = "/status",
    tag = "Status",
    responses(
        (status = 200, description = "Application feature availability", body = StatusResponse),
    )
)]
pub async fn status(State(state): State<InvoiceState>) -> Json<StatusResponse> {
    let invoice_available = state.superuser_session.read().await.is_some();
    let sumup_available = state.config.sumup.is_some();
    Json(StatusResponse { invoice_available, sumup_available })
}
