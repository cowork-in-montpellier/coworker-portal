use std::collections::HashMap;
use sqlx::FromRow;

use crate::invoice::domain::VoucherStatus;
use crate::invoice::state::State;

#[derive(FromRow)]
struct ValidVoucherRow {
    unify_id: String,
    unify_create_time: i64,
}

pub async fn run(state: &State) {
    tracing::info!("Voucher sync: starting");

    let rows = match sqlx::query_as::<_, ValidVoucherRow>(
        "SELECT unify_id, unify_create_time FROM portal_voucher WHERE status = 'Valid'",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Voucher sync: failed to fetch valid vouchers");
            return;
        }
    };

    tracing::info!(count = rows.len(), "Voucher sync: valid vouchers loaded");

    if rows.is_empty() {
        tracing::info!("Voucher sync: no valid vouchers, skipping");
        return;
    }

    // Group by unify_create_time to minimise Unify API calls (one call per batch).
    let mut by_create_time: HashMap<i64, Vec<String>> = HashMap::new();
    for row in rows {
        by_create_time.entry(row.unify_create_time).or_default().push(row.unify_id);
    }

    tracing::info!(
        valid_vouchers = by_create_time.values().map(|v| v.len()).sum::<usize>(),
        batches = by_create_time.len(),
        "Voucher sync: grouped into Unify batches"
    );

    let mut updated = 0usize;
    let mut failed_batches = 0usize;

    for (create_time, unify_ids) in &by_create_time {
        tracing::debug!(
            create_time,
            voucher_count = unify_ids.len(),
            ids = ?unify_ids,
            "Voucher sync: querying Unify batch"
        );

        let statuses = match state.unify.get_vouchers_status(*create_time, "", unify_ids).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(create_time, error = %e, "Voucher sync: Unify call failed for batch");
                failed_batches += 1;
                continue;
            }
        };

        tracing::debug!(
            create_time,
            returned = statuses.len(),
            "Voucher sync: Unify batch response received"
        );

        for (unify_id, status) in &statuses {
            tracing::debug!(%unify_id, status = status.as_str(), "Voucher sync: status from Unify");

            if *status != VoucherStatus::Valid {
                tracing::info!(
                    %unify_id,
                    new_status = status.as_str(),
                    "Voucher sync: voucher is no longer valid"
                );
            }

            match sqlx::query("UPDATE portal_voucher SET status = $1 WHERE unify_id = $2")
                .bind(status.as_str())
                .bind(unify_id)
                .execute(&state.db)
                .await
            {
                Ok(_) => updated += 1,
                Err(e) => tracing::error!(%unify_id, error = %e, "Voucher sync: DB update failed"),
            }
        }
    }

    tracing::info!(updated, failed_batches, "Voucher sync: done");
}
