use std::collections::{HashMap, HashSet};

use chrono::Timelike as _;
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

    // 2. Compute hours elapsed since midnight (Paris) so we only capture today's connections.
    //    Using a fixed 24h window would bleed into the previous day when the task runs early.
    //    hour() is 0-based, so +1 gives the ceiling of elapsed hours (min 1 at 00:xx).
    let now_paris = chrono::Utc::now().with_timezone(&Paris);
    let within_hours = now_paris.hour() + 1;
    tracing::info!(within_hours, "Monthly usage diary: querying Unify guests since today midnight");

    let guests = match state.unify.get_active_guests(within_hours).await {
        Ok(g) => g,
        Err(e) => {
            tracing::error!(error = %e, "Monthly usage diary: Unify guest query failed");
            return;
        }
    };

    tracing::info!(total_guests = guests.len(), "Monthly usage diary: guests returned by Unify");

    // 3. Keep only guests whose voucher_id is one of our monthly vouchers.
    //    Group by voucher_id, collecting distinct MACs to count connected devices.
    let mut by_voucher: HashMap<String, HashSet<String>> = HashMap::new();
    for guest in guests {
        if monthly_ids.contains(&guest.voucher_id) {
            by_voucher.entry(guest.voucher_id).or_default().insert(guest.mac);
        }
    }

    tracing::info!(
        active_monthly = by_voucher.len(),
        "Monthly usage diary: monthly vouchers with at least one active guest today"
    );

    if by_voucher.is_empty() {
        tracing::info!("Monthly usage diary: no monthly voucher connections today, nothing to record");
        return;
    }

    // 4. Append today's date to active_days for each active voucher (idempotent — skip if already present).
    let today = chrono::Utc::now().with_timezone(&Paris).date_naive();

    let mut recorded = 0usize;
    for (unify_id, macs) in &by_voucher {
        tracing::info!(%unify_id, %today, device_count = macs.len(), "Monthly usage diary: recording active day");

        match sqlx::query(
            "UPDATE portal_voucher SET active_days = array_append(active_days, $1) WHERE unify_id = $2 AND NOT ($1 = ANY(active_days))",
        )
        .bind(today)
        .bind(unify_id)
        .execute(&state.db)
        .await
        {
            Ok(_) => recorded += 1,
            Err(e) => tracing::error!(%unify_id, error = %e, "Monthly usage diary: DB update failed"),
        }
    }

    tracing::info!(recorded, date = %today, "Monthly usage diary: done");
}
