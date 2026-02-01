//! dYdX v4 Client
//!
//! Low-level client for dYdX v4 indexer API.

use reqwest::Client;
use serde_json::Value;

use crate::connectors::ConnectorError;

/// dYdX v4 API client
pub struct DydxClient {
    client: Client,
    indexer_url: String,
}

impl DydxClient {
    /// Create a new client
    pub fn new(indexer_url: &str) -> Self {
        Self {
            client: Client::new(),
            indexer_url: indexer_url.to_string(),
        }
    }

    /// Get perpetual markets
    pub async fn get_markets(&self) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!("{}/perpetualMarkets", self.indexer_url))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get market by ticker
    pub async fn get_market(&self, ticker: &str) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!("{}/perpetualMarkets/{}", self.indexer_url, ticker))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get orderbook
    pub async fn get_orderbook(&self, market: &str) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/orderbooks/perpetualMarket/{}",
                self.indexer_url, market
            ))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get trades
    pub async fn get_trades(&self, market: &str, limit: u32) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/trades/perpetualMarket/{}?limit={}",
                self.indexer_url, market, limit
            ))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get subaccount
    pub async fn get_subaccount(
        &self,
        address: &str,
        subaccount_number: u32,
    ) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/addresses/{}/subaccountNumber/{}",
                self.indexer_url, address, subaccount_number
            ))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get orders
    pub async fn get_orders(
        &self,
        address: &str,
        subaccount_number: u32,
    ) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/orders?address={}&subaccountNumber={}",
                self.indexer_url, address, subaccount_number
            ))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get fills
    pub async fn get_fills(
        &self,
        address: &str,
        subaccount_number: u32,
    ) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/fills?address={}&subaccountNumber={}",
                self.indexer_url, address, subaccount_number
            ))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get historical funding
    pub async fn get_historical_funding(&self, market: &str) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/historicalFunding/{}",
                self.indexer_url, market
            ))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }

    /// Get candles
    pub async fn get_candles(
        &self,
        market: &str,
        resolution: &str,
    ) -> Result<Value, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/candles/perpetualMarkets/{}?resolution={}",
                self.indexer_url, market, resolution
            ))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        Ok(data)
    }
}
