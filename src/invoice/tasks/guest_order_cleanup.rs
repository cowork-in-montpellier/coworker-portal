use crate::invoice::state::State;

pub async fn run(state: &State) {
    match sqlx::query(
        "DELETE FROM portal_guest_pending_order WHERE created_at < NOW() - INTERVAL '3 months'",
    )
    .execute(&state.db)
    .await
    {
        Ok(r) => tracing::info!(deleted = r.rows_affected(), "Guest order cleanup: old rows removed"),
        Err(e) => tracing::error!(error = %e, "Guest order cleanup failed"),
    }
}
