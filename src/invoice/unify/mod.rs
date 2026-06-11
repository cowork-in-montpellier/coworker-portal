pub mod mock;
pub mod real;

use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;

use crate::invoice::domain::VoucherStatus;

pub struct CreateVouchersRequest {
    pub n: i32,
    pub duration_hours: i32,
    pub note: String,
    pub quota: i32,
}

pub struct UnifyVoucher {
    pub unify_id: String,
    pub code: String,
    pub duration: i32,       // hours
    pub create_time: i64,    // Unix timestamp
}

/// A guest device currently authorized via a voucher.
pub struct ActiveGuest {
    pub voucher_id: String,  // unify_id (_id) of the voucher used
    pub mac: String,         // device MAC address
}

#[async_trait]
pub trait UnifyClient: Send + Sync {
    /// Provision vouchers on Unify and return the created vouchers.
    async fn create_vouchers(&self, req: CreateVouchersRequest) -> Result<Vec<UnifyVoucher>>;

    /// Fetch live status for a set of vouchers identified by their Unify IDs.
    async fn get_vouchers_status(
        &self,
        create_time: i64,
        note: &str,
        unify_ids: &[String],
    ) -> Result<HashMap<String, VoucherStatus>>;

    /// Fetch guest devices that connected via a voucher within the last `within_hours` hours.
    /// Only returns guests that have a voucher_id (i.e. authorized via voucher).
    async fn get_active_guests(&self, within_hours: u32) -> Result<Vec<ActiveGuest>>;

    /// Revoke (delete) a voucher on Unify by its `_id`.
    async fn revoke_voucher(&self, unify_id: &str) -> Result<()>;
}
