use anyhow::{Context, Result};
use serde::Deserialize;

use super::config::SumUpConfig;

pub struct SumUpClient {
    client: reqwest::Client,
    base_url: String,
    merchant_code: String,
}

pub struct CheckoutCreated {
    pub checkout_id: String,
    pub checkout_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckoutStatus {
    Pending,
    Paid,
    Failed,
}

#[derive(Deserialize)]
struct CreateCheckoutResponse {
    id: String,
    hosted_checkout_url: Option<String>,
}

#[derive(Deserialize)]
struct GetCheckoutResponse {
    status: String,
}

impl SumUpClient {
    pub fn new(config: &SumUpConfig) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        let auth = format!("Bearer {}", config.api_key);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            auth.parse().context("Invalid SumUp API key format")?,
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self {
            client,
            base_url: config.base_url.clone(),
            merchant_code: config.merchant_code.clone(),
        })
    }

    pub async fn create_checkout(
        &self,
        checkout_reference: &str,
        description: &str,
        amount: f64,
        redirect_url: &str,
        return_url: &str,
    ) -> Result<CheckoutCreated> {
        let body = serde_json::json!({
            "checkout_reference": checkout_reference,
            "description": description,
            "amount": amount,
            "currency": "EUR",
            "merchant_code": self.merchant_code,
            "redirect_url": redirect_url,
            "return_url": return_url,
            "hosted_checkout": { "enabled": true },
        });
        let url = format!("{}/v0.1/checkouts", self.base_url);
        tracing::info!(
            %url,
            checkout_reference,
            description,
            amount,
            currency = "EUR",
            merchant_code = %self.merchant_code,
            %redirect_url,
            %return_url,
            "Creating SumUp checkout",
        );
        let resp: CreateCheckoutResponse = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .context("SumUp create checkout failed")?
            .json()
            .await?;

        let checkout_url = resp
            .hosted_checkout_url
            .context("SumUp response missing hosted_checkout_url")?;
        tracing::info!(checkout_id = %resp.id, %checkout_url, "SumUp checkout created");
        Ok(CheckoutCreated { checkout_id: resp.id, checkout_url })
    }

    pub async fn get_checkout(&self, checkout_id: &str) -> Result<CheckoutStatus> {
        let url = format!("{}/v0.1/checkouts/{}", self.base_url, checkout_id);
        tracing::debug!(%url, checkout_id, "Fetching SumUp checkout status");
        let resp: GetCheckoutResponse = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()
            .context("SumUp get checkout failed")?
            .json()
            .await?;
        let status = match resp.status.to_uppercase().as_str() {
            "PAID" => CheckoutStatus::Paid,
            "FAILED" => CheckoutStatus::Failed,
            _ => CheckoutStatus::Pending,
        };
        tracing::debug!(checkout_id, raw_status = %resp.status, ?status, "SumUp checkout status");
        Ok(status)
    }
}
