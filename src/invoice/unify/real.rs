use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

use crate::invoice::{config::UnifyConfig, domain::VoucherStatus};
use super::{CreateVouchersRequest, UnifyClient, UnifyVoucher};

pub struct RealUnifyClient {
    client: reqwest::Client,
    base_url: String,
    site: String,
    config: UnifyConfig,
}

impl RealUnifyClient {
    pub async fn new(config: &UnifyConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .build()?;

        let this = Self {
            client,
            base_url: config.base_url.clone(),
            site: config.site.clone(),
            config: config.clone(),
        };
        this.login(&config.username, &config.password).await?;
        Ok(this)
    }

    async fn login(&self, username: &str, password: &str) -> Result<()> {
        self.client
            .post(format!("{}/api/login", self.base_url))
            .json(&serde_json::json!({ "username": username, "password": password, "site_name": "default", "for_hotspot": "true" }))
            .send()
            .await?
            .error_for_status()?;
        tracing::info!("Unify login successful as {username}");
        Ok(())
    }

    /// Send a request, re-login once on 401 and retry.
    async fn send_with_retry<F>(&self, build: F) -> Result<reqwest::Response>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let resp = build().send().await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            tracing::info!("Unify session expired, re-authenticating");
            self.login(&self.config.username, &self.config.password).await?;
            Ok(build().send().await?.error_for_status()?)
        } else {
            Ok(resp.error_for_status()?)
        }
    }

    /// Fetch the MAC addresses of hotspot clients currently online.
    async fn connected_macs(&self) -> Result<std::collections::HashSet<String>> {
        let url = format!("{}/v2/api/site/{}/hotspot/clients", self.base_url, self.site);
        tracing::debug!(%url, "Querying Unify hotspot clients");
        let raw = self
            .send_with_retry(|| self.client.get(&url).query(&[("withinHours", "1")]))
            .await?;
        let status = raw.status();
        let body = raw.text().await?;
        tracing::debug!(%status, body = %body, "Unify hotspot clients raw response");
        let clients: Vec<HotspotClientDto> = serde_json::from_str(&body)
            .map_err(|e| anyhow::anyhow!("hotspot/clients parse error: {e} — body: {body}"))?;
        let online: std::collections::HashSet<String> = clients
            .into_iter()
            .filter(|c| c.status.as_deref() == Some("online"))
            .map(|c| c.mac)
            .collect();
        tracing::debug!(total = online.len(), "Unify hotspot clients online");
        Ok(online)
    }

    async fn macs_for_voucher(&self, unify_id: &str) -> Result<Vec<String>> {
        let url = format!("{}/api/s/{}/stat/guest", self.base_url, self.site);
        // POST with `within` (hours) per the Unify API spec; 720h = 30 days
        let body = serde_json::json!({ "within": 720 });
        let resp: GuestListResponse = self
            .send_with_retry(|| self.client.post(&url).json(&body))
            .await?
            .json()
            .await?;
        let macs = resp.data.into_iter()
            .filter(|g| !g.expired)
            .filter(|g| g.voucher_id.as_deref() == Some(unify_id))
            .map(|g| g.mac)
            .collect();
        Ok(macs)
    }

    async fn unauthorize_guest(&self, mac: &str) -> Result<()> {
        let body = serde_json::json!({ "cmd": "unauthorize-guest", "mac": mac });
        let url = format!("{}/api/s/{}/cmd/stamgr", self.base_url, self.site);
        tracing::debug!(%url, %mac, "Unauthorizing Unify guest");
        self.send_with_retry(|| self.client.post(&url).json(&body)).await?;
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
struct GuestDto {
    mac: String,
    minutes: Option<i32>,
    voucher_id: Option<String>,
    expired: bool,
}

#[derive(Deserialize, Debug)]
struct GuestListResponse {
    data: Vec<GuestDto>,
}

#[derive(Deserialize, Debug)]
struct HotspotClientDto {
    mac: String,
    status: Option<String>,
}

#[derive(Deserialize, Debug)]
struct UnifyVoucherDto {
    #[serde(rename = "_id")]
    id: String,
    code: String,
    duration: i32,       // minutes
    note: String,
    status: String,
    status_expires: Option<i64>,
}

#[derive(Deserialize, Debug)]
struct VoucherListResponse {
    data: Vec<UnifyVoucherDto>,
}

#[derive(Deserialize, Debug)]
struct CreateVoucherResponseItem {
    create_time: i64,
}

#[derive(Deserialize)]
struct CreateVoucherResponse {
    data: Vec<CreateVoucherResponseItem>,
}

fn map_status(dto: &UnifyVoucherDto) -> VoucherStatus {
    // status_expires = 0 means the voucher hasn't been activated yet — not expired.
    // Only treat it as expired when Unify explicitly says so via the status string.
    tracing::debug!(voucher_id=&dto.id, voucher_status=&dto.status, "Mapping voucher status");
    match dto.status.to_uppercase().as_str() {
        "VALID_ONE" | "VALID_MULTI" => VoucherStatus::Valid,
        "USED_MULTIPLE" | "EXPIRED" => VoucherStatus::Used,
        _ => VoucherStatus::Unknown,
    }
}

#[async_trait]
impl UnifyClient for RealUnifyClient {
    async fn create_vouchers(&self, req: CreateVouchersRequest) -> Result<Vec<UnifyVoucher>> {
        // Step 1: create the batch, get create_time
        let body = serde_json::json!({
            "cmd": "create-voucher",
            "n": req.n,
            "quota": req.quota,
            "expire_number": req.duration_hours,
            "expire_unit": 60,  // hour multiplier
            "note": req.note,
        });
        let url = format!("{}/api/s/{}/cmd/hotspot", self.base_url, self.site);
        let create_resp: CreateVoucherResponse = self
            .send_with_retry(|| self.client.post(&url).json(&body))
            .await?
            .json().await?;

        let create_time = create_resp.data.first()
            .map(|r| r.create_time)
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        // Step 2: retrieve the batch by create_time and filter by note
        let list_body = serde_json::json!({ "create_time": create_time });
        let list_url = format!("{}/api/s/{}/stat/voucher", self.base_url, self.site);
        let list_resp: VoucherListResponse = self
            .send_with_retry(|| self.client.post(&list_url).json(&list_body))
            .await?
            .json().await?;

        let vouchers = list_resp.data.into_iter()
            .filter(|v| v.note.eq(&req.note))
            .map(|v| UnifyVoucher {
                unify_id: v.id,
                code: v.code,
                duration: v.duration / 60, // minutes → hours
                create_time,
            })
            .collect();

        Ok(vouchers)
    }

    async fn get_vouchers_status(
        &self,
        create_time: i64,
        _note: &str,
        unify_ids: &[String],
    ) -> Result<HashMap<String, VoucherStatus>> {
        let body = serde_json::json!({ "create_time": create_time });
        let url = format!("{}/api/s/{}/stat/voucher", self.base_url, self.site);
        tracing::debug!(%url, %body, "Querying Unify voucher status");
        let resp: VoucherListResponse = self
            .send_with_retry(|| self.client.post(&url).json(&body))
            .await?
            .json().await?;

        tracing::debug!(
            create_time,
            total_in_batch = resp.data.len(),
            looking_for = ?unify_ids,
            "Unify voucher status response"
        );
        for v in &resp.data {
            tracing::debug!(
                id = %v.id,
                code = %v.code,
                status = %v.status,
                status_expires = ?v.status_expires,
                note = %v.note,
                "Unify voucher"
            );
        }

        let id_set: std::collections::HashSet<&str> =
            unify_ids.iter().map(|s| s.as_str()).collect();

        let mut map: HashMap<String, VoucherStatus> = resp.data.into_iter()
            .filter(|v| id_set.contains(v.id.as_str()))
            .map(|v| (v.id.clone(), map_status(&v)))
            .collect();

        // Vouchers absent from the response have been revoked — treat as Expired.
        for id in unify_ids {
            map.entry(id.clone()).or_insert(VoucherStatus::Expired);
        }

        Ok(map)
    }

    async fn revoke_voucher(&self, unify_id: &str) -> Result<()> {
        // Find which MACs are currently authenticated with this voucher before deleting it.
        let macs = self.macs_for_voucher(unify_id).await.unwrap_or_else(|e| {
            tracing::warn!(unify_id, error = %e, "Could not query guest MACs before revocation; will skip unauthorize step");
            vec![]
        });

        let body = serde_json::json!({ "cmd": "delete-voucher", "_id": unify_id });
        let url = format!("{}/api/s/{}/cmd/hotspot", self.base_url, self.site);
        tracing::debug!(%url, unify_id, "Revoking Unify voucher");
        self.send_with_retry(|| self.client.post(&url).json(&body)).await?;

        // Terminate each active guest session so clients are immediately kicked off the network.
        for mac in &macs {
            if let Err(e) = self.unauthorize_guest(mac).await {
                tracing::warn!(%mac, error = %e, "Failed to unauthorize guest after voucher revocation");
            }
        }
        Ok(())
    }

    async fn get_active_guests(&self) -> Result<Vec<super::ActiveGuest>> {
        let url = format!("{}/api/s/{}/stat/guest", self.base_url, self.site);
        // POST with `within` (hours) per the Unify API spec; 720h = 30 days
        let guest_body = serde_json::json!({ "within": 720 });
        tracing::debug!(%url, "Querying Unify active guests (30-day window)");

        let (resp, connected) = tokio::try_join!(
            async {
                let r: GuestListResponse = self
                    .send_with_retry(|| self.client.post(&url).json(&guest_body))
                    .await?
                    .json()
                    .await?;
                anyhow::Ok(r)
            },
            self.connected_macs(),
        )?;

        tracing::debug!(total = resp.data.len(), connected_stations = connected.len(), "Unify guests + stations response");

        // Group by voucher_id, collecting all MACs currently on the AP for each voucher.
        let mut by_voucher: std::collections::HashMap<String, super::ActiveGuest> =
            std::collections::HashMap::new();

        for g in resp.data.into_iter()
            .filter(|g| g.expired == false)
            .filter(|g| connected.contains(&g.mac)) {
            if let Some(vid) = g.voucher_id {
                let entry = by_voucher.entry(vid.clone()).or_insert_with(|| {
                    tracing::debug!(voucher_id = %vid, minutes = ?g.minutes, "Unify active voucher (physically associated)");
                    super::ActiveGuest {
                        voucher_id: vid,
                        macs: vec![],
                        minutes: g.minutes,
                    }
                });
                entry.macs.push(g.mac);
            }
        }

        let guests: Vec<_> = by_voucher.into_values().collect();
        tracing::debug!(active_vouchers = guests.len(), "Unify active guests grouped by voucher");
        Ok(guests)
    }
}
