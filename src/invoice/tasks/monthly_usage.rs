use std::collections::HashSet;

use chrono_tz::Europe::Paris;

use crate::invoice::state::State;

pub async fn run(state: &State) {
    tracing::info!("Monthly usage diary: starting");

    // 1. Load all unify_ids that belong to Monthly-type vouchers.
    let monthly_ids: HashSet<String> = match sqlx::query_scalar::<_, String>(
        r#"
        SELECT pv.unify_id
        FROM portal_voucher pv
        JOIN billjobs_billline bl ON bl.id = pv.billline_id
        JOIN portal_service ps   ON ps.external_service_id = bl.service_id
        WHERE ps.kind = 'Monthly'
        "#,
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(ids) => ids.into_iter().collect(),
        Err(e) => {
            tracing::error!(error = %e, "Monthly usage diary: failed to fetch monthly voucher IDs");
            return;
        }
    };

    tracing::info!(count = monthly_ids.len(), "Monthly usage diary: monthly vouchers found");

    if monthly_ids.is_empty() {
        tracing::info!("Monthly usage diary: no monthly vouchers, skipping");
        return;
    }

    // 2. Fetch currently connected vouchers (30-day window, mac-filtered).
    let guests = match state.unify.get_active_guests().await {
        Ok(g) => g,
        Err(e) => {
            tracing::error!(error = %e, "Monthly usage diary: Unify guest query failed");
            return;
        }
    };

    tracing::info!(active_vouchers = guests.len(), "Monthly usage diary: active vouchers returned by Unify");

    // 3. Keep only monthly vouchers.
    let active_monthly: Vec<_> = guests.into_iter()
        .filter(|g| monthly_ids.contains(&g.voucher_id))
        .collect();

    tracing::info!(
        active_monthly = active_monthly.len(),
        "Monthly usage diary: monthly vouchers with at least one active guest today"
    );

    if active_monthly.is_empty() {
        tracing::info!("Monthly usage diary: no monthly voucher connections today, nothing to record");
        return;
    }

    // 4. Append today's date to active_days for each active voucher (idempotent).
    let today = chrono::Utc::now().with_timezone(&Paris).date_naive();

    let mut recorded = 0usize;
    for guest in &active_monthly {
        tracing::info!(unify_id = %guest.voucher_id, %today, device_count = guest.macs.len(), "Monthly usage diary: recording active day");

        match sqlx::query(
            "UPDATE portal_voucher SET active_days = array_append(active_days, $1) WHERE unify_id = $2 AND NOT ($1 = ANY(active_days))",
        )
        .bind(today)
        .bind(&guest.voucher_id)
        .execute(&state.db)
        .await
        {
            Ok(_) => recorded += 1,
            Err(e) => tracing::error!(unify_id = %guest.voucher_id, error = %e, "Monthly usage diary: DB update failed"),
        }
    }

    tracing::info!(recorded, date = %today, "Monthly usage diary: done");
}
