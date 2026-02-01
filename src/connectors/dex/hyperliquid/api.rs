//! Hyperliquid REST API client
//!
//! Low-level API client for Hyperliquid REST endpoints.

use reqwest::Client;
use serde_json::Value;

use crate::connectors::ConnectorError;

/// Hyperliquid API client
pub struct HyperliquidApi {
    client: Client,
    base_url: String,
}

impl HyperliquidApi {
    /// Create a new API client
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Make an info request (POST /info)
    pub async fn info(&self, request: Value) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .post(&format!("{}/info", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ConnectorError::HttpError(
                resp.error_for_status().unwrap_err(),
            ));
        }

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Make an exchange request (POST /exchange)
    pub async fn exchange(&self, request: Value) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .post(&format!("{}/exchange", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ConnectorError::HttpError(
                resp.error_for_status().unwrap_err(),
            ));
        }

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get all mid prices
    pub async fn get_all_mids(&self) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "allMids"
        }))
        .await
    }

    /// Get meta info
    pub async fn get_meta(&self) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "meta"
        }))
        .await
    }

    /// Get L2 order book
    pub async fn get_l2_book(&self, coin: &str) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "l2Book",
            "coin": coin
        }))
        .await
    }

    /// Get recent trades
    pub async fn get_recent_trades(&self, coin: &str) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "recentTrades",
            "coin": coin
        }))
        .await
    }

    /// Get user state
    pub async fn get_user_state(&self, user: &str) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "clearinghouseState",
            "user": user
        }))
        .await
    }

    /// Get open orders
    pub async fn get_open_orders(&self, user: &str) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "openOrders",
            "user": user
        }))
        .await
    }

    /// Get order status
    pub async fn get_order_status(&self, user: &str, oid: u64) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "orderStatus",
            "user": user,
            "oid": oid
        }))
        .await
    }

    /// Get user fills
    pub async fn get_user_fills(&self, user: &str) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "userFills",
            "user": user
        }))
        .await
    }

    /// Get funding history
    pub async fn get_funding_history(
        &self,
        coin: &str,
        start_time: i64,
        end_time: Option<i64>,
    ) -> Result<Value, ConnectorError> {
        let mut request = serde_json::json!({
            "type": "fundingHistory",
            "coin": coin,
            "startTime": start_time
        });

        if let Some(end) = end_time {
            request["endTime"] = serde_json::json!(end);
        }

        self.info(request).await
    }

    /// Get candlestick data
    pub async fn get_candles(
        &self,
        coin: &str,
        interval: &str,
        start_time: i64,
        end_time: i64,
    ) -> Result<Value, ConnectorError> {
        self.info(serde_json::json!({
            "type": "candleSnapshot",
            "req": {
                "coin": coin,
                "interval": interval,
                "startTime": start_time,
                "endTime": end_time
            }
        }))
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_creation() {
        let api = HyperliquidApi::new("https://api.hyperliquid.xyz");
        assert!(api.base_url.contains("hyperliquid"));
    }
}
