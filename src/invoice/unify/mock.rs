use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use anyhow::Result;
use async_trait::async_trait;
use rand::Rng;

use crate::invoice::domain::VoucherStatus;
use super::{ActiveGuest, CreateVouchersRequest, UnifyClient, UnifyVoucher};

/// Mimics Unify without a real controller. Status is tracked deterministically
/// (a voucher is Valid until `revoke_voucher` is called on it) rather than
/// randomized, so local testing of revoke/split flows reflects real actions
/// instead of noise.
#[derive(Default)]
pub struct MockUnifyClient {
    revoked: Mutex<HashSet<String>>,
}

#[async_trait]
impl UnifyClient for MockUnifyClient {
    async fn create_vouchers(&self, req: CreateVouchersRequest) -> Result<Vec<UnifyVoucher>> {
        let mut rng = rand::thread_rng();
        let create_time = chrono::Utc::now().timestamp();

        let vouchers = (0..req.n)
            .map(|_| {
                let code = format!("{:010}", rng.gen_range(0u64..10_000_000_000u64));
                let unify_id = uuid::Uuid::new_v4().to_string();
                tracing::debug!(note = %req.note, unify_id = %unify_id, "mock: created voucher");
                UnifyVoucher { unify_id, code, duration: req.duration_hours, create_time }
            })
            .collect();

        Ok(vouchers)
    }

    async fn get_vouchers_status(
        &self,
        _create_time: i64,
        _note: &str,
        unify_ids: &[String],
    ) -> Result<HashMap<String, VoucherStatus>> {
        let revoked = self.revoked.lock().unwrap();
        Ok(unify_ids
            .iter()
            .map(|id| {
                let status = if revoked.contains(id) {
                    VoucherStatus::Expired
                } else {
                    VoucherStatus::Valid
                };
                tracing::debug!(unify_id = %id, status = ?status, "mock: voucher status");
                (id.clone(), status)
            })
            .collect())
    }

    async fn revoke_voucher(&self, unify_id: &str) -> Result<()> {
        self.revoked.lock().unwrap().insert(unify_id.to_string());
        tracing::debug!(unify_id = %unify_id, "mock: revoked voucher");
        Ok(())
    }

    async fn get_active_guests(&self) -> Result<Vec<ActiveGuest>> {
        tracing::debug!("mock: get_active_guests — returning empty list");
        Ok(vec![])
    }
}
