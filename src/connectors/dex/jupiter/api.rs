//! Jupiter API Client
//!
//! Low-level client for Jupiter swap aggregator API.

use reqwest::Client;
use serde_json::Value;

use crate::connectors::ConnectorError;

/// Jupiter API client
pub struct JupiterApi {
    client: Client,
    quote_url: String,
    price_url: String,
}

impl JupiterApi {
    /// Create a new API client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            quote_url: "https://quote-api.jup.ag/v6".to_string(),
            price_url: "https://price.jup.ag/v6".to_string(),
        }
    }

    /// Get a quote for a swap
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
        slippage_bps: u32,
    ) -> Result<Value, ConnectorError> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.quote_url, input_mint, output_mint, amount, slippage_bps
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            return Err(ConnectorError::InvalidResponse(error));
        }

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get swap transaction
    pub async fn get_swap(
        &self,
        quote: Value,
        user_public_key: &str,
    ) -> Result<Value, ConnectorError> {
        let body = serde_json::json!({
            "quoteResponse": quote,
            "userPublicKey": user_public_key,
            "wrapAndUnwrapSol": true,
            "dynamicComputeUnitLimit": true,
            "prioritizationFeeLamports": "auto"
        });

        let resp = self
            .client
            .post(&format!("{}/swap", self.quote_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error = resp.text().await.unwrap_or_default();
            return Err(ConnectorError::InvalidResponse(error));
        }

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get token price
    pub async fn get_price(&self, mint: &str) -> Result<Value, ConnectorError> {
        let url = format!("{}/price?ids={}", self.price_url, mint);

        let resp = self.client.get(&url).send().await?;
        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get multiple token prices
    pub async fn get_prices(&self, mints: &[&str]) -> Result<Value, ConnectorError> {
        let ids = mints.join(",");
        let url = format!("{}/price?ids={}", self.price_url, ids);

        let resp = self.client.get(&url).send().await?;
        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get token list
    pub async fn get_tokens(&self) -> Result<Value, ConnectorError> {
        let url = "https://token.jup.ag/all";

        let resp = self.client.get(url).send().await?;
        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get strict token list (verified tokens only)
    pub async fn get_strict_tokens(&self) -> Result<Value, ConnectorError> {
        let url = "https://token.jup.ag/strict";

        let resp = self.client.get(url).send().await?;
        let data: Value = resp.json().await?;
        Ok(data)
    }
}

impl Default for JupiterApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_creation() {
        let api = JupiterApi::new();
        assert!(api.quote_url.contains("jup.ag"));
    }
}
