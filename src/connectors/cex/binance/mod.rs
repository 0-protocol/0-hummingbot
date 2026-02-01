//! Binance Exchange Connector
//!
//! Full implementation for Binance Spot and Perpetual trading.

mod rest;
mod ws;

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::connectors::{
    auth::ApiCredentials,
    error::ConnectorError,
    types::*,
    ConnectorBase, ExchangeType,
};

pub use rest::BinanceRestClient;
pub use ws::BinanceWsClient;

/// Binance connector supporting both Spot and Perpetual markets
pub struct BinanceConnector {
    rest: BinanceRestClient,
    ws: Arc<RwLock<Option<BinanceWsClient>>>,
    exchange_type: ExchangeType,
}

impl BinanceConnector {
    pub fn spot(api_key: &str, api_secret: &str) -> Self {
        Self {
            rest: BinanceRestClient::spot(ApiCredentials::new(api_key, api_secret)),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Spot,
        }
    }
    
    pub fn perpetual(api_key: &str, api_secret: &str) -> Self {
        Self {
            rest: BinanceRestClient::perpetual(ApiCredentials::new(api_key, api_secret)),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Perpetual,
        }
    }
    
    pub fn spot_testnet(api_key: &str, api_secret: &str) -> Self {
        Self {
            rest: BinanceRestClient::spot_testnet(ApiCredentials::new(api_key, api_secret)),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Spot,
        }
    }
    
    pub fn perpetual_testnet(api_key: &str, api_secret: &str) -> Self {
        Self {
            rest: BinanceRestClient::perpetual_testnet(ApiCredentials::new(api_key, api_secret)),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Perpetual,
        }
    }
    
    pub fn spot_public() -> Self {
        Self {
            rest: BinanceRestClient::spot_public(),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Spot,
        }
    }
    
    pub fn perpetual_public() -> Self {
        Self {
            rest: BinanceRestClient::perpetual_public(),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Perpetual,
        }
    }
    
    async fn get_ws(&self) -> Result<BinanceWsClient, ConnectorError> {
        let ws_read = self.ws.read().await;
        if let Some(ws) = ws_read.as_ref() {
            return Ok(ws.clone());
        }
        drop(ws_read);
        
        let mut ws_write = self.ws.write().await;
        if ws_write.is_none() {
            let ws = match self.exchange_type {
                ExchangeType::Spot => BinanceWsClient::spot(),
                ExchangeType::Perpetual => BinanceWsClient::perpetual(),
                ExchangeType::SpotAndPerpetual => BinanceWsClient::spot(),
            };
            *ws_write = Some(ws);
        }
        
        Ok(ws_write.as_ref().unwrap().clone())
    }
}

#[async_trait]
impl ConnectorBase for BinanceConnector {
    fn name(&self) -> &str {
        "binance"
    }
    
    fn exchange_type(&self) -> ExchangeType {
        self.exchange_type
    }
    
    async fn get_ticker(&self, pair: &str) -> Result<Ticker, ConnectorError> {
        self.rest.get_ticker(pair).await
    }
    
    async fn get_orderbook(&self, pair: &str, depth: u32) -> Result<OrderBook, ConnectorError> {
        self.rest.get_orderbook(pair, depth).await
    }
    
    async fn get_trades(&self, pair: &str, limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        self.rest.get_trades(pair, limit).await
    }
    
    async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        self.rest.place_order(order).await
    }
    
    async fn cancel_order(&self, pair: &str, order_id: &str) -> Result<CancelResponse, ConnectorError> {
        self.rest.cancel_order(pair, order_id).await
    }
    
    async fn get_order(&self, pair: &str, order_id: &str) -> Result<Order, ConnectorError> {
        self.rest.get_order(pair, order_id).await
    }
    
    async fn get_open_orders(&self, pair: Option<&str>) -> Result<Vec<Order>, ConnectorError> {
        self.rest.get_open_orders(pair).await
    }
    
    async fn cancel_all_orders(&self, pair: Option<&str>) -> Result<u32, ConnectorError> {
        self.rest.cancel_all_orders(pair).await
    }
    
    async fn get_balance(&self, asset: &str) -> Result<Balance, ConnectorError> {
        self.rest.get_balance(asset).await
    }
    
    async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        self.rest.get_balances().await
    }
    
    async fn get_positions(&self, pair: Option<&str>) -> Result<Vec<Position>, ConnectorError> {
        self.rest.get_positions(pair).await
    }
    
    async fn subscribe_ticker(&self, pair: &str) -> Result<TickerStream, ConnectorError> {
        let ws = self.get_ws().await?;
        ws.subscribe_ticker(pair).await
    }
    
    async fn subscribe_orderbook(&self, pair: &str) -> Result<OrderBookStream, ConnectorError> {
        let ws = self.get_ws().await?;
        ws.subscribe_orderbook(pair).await
    }
    
    async fn subscribe_trades(&self, pair: &str) -> Result<TradeStream, ConnectorError> {
        let ws = self.get_ws().await?;
        ws.subscribe_trades(pair).await
    }
    
    async fn subscribe_user_data(&self) -> Result<UserDataStream, ConnectorError> {
        let listen_key = self.rest.get_listen_key().await?;
        let ws = self.get_ws().await?;
        ws.subscribe_user_data(&listen_key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connector_creation() {
        let spot = BinanceConnector::spot_public();
        assert_eq!(spot.name(), "binance");
        assert_eq!(spot.exchange_type(), ExchangeType::Spot);
        
        let perp = BinanceConnector::perpetual_public();
        assert_eq!(perp.name(), "binance");
        assert_eq!(perp.exchange_type(), ExchangeType::Perpetual);
    }
}
