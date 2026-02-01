//! OKX Exchange Connector
//!
//! Full implementation for OKX Spot and Perpetual trading.

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

pub use rest::OkxRestClient;
pub use ws::OkxWsClient;

/// OKX connector supporting both Spot and Perpetual markets
pub struct OkxConnector {
    rest: OkxRestClient,
    ws: Arc<RwLock<Option<OkxWsClient>>>,
    exchange_type: ExchangeType,
    inst_type: String,
}

impl OkxConnector {
    pub fn spot(api_key: &str, api_secret: &str, passphrase: &str) -> Self {
        Self {
            rest: OkxRestClient::new(ApiCredentials::with_passphrase(api_key, api_secret, passphrase)),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Spot,
            inst_type: "SPOT".to_string(),
        }
    }
    
    pub fn perpetual(api_key: &str, api_secret: &str, passphrase: &str) -> Self {
        Self {
            rest: OkxRestClient::new(ApiCredentials::with_passphrase(api_key, api_secret, passphrase)),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Perpetual,
            inst_type: "SWAP".to_string(),
        }
    }
    
    pub fn spot_demo(api_key: &str, api_secret: &str, passphrase: &str) -> Self {
        Self {
            rest: OkxRestClient::demo(ApiCredentials::with_passphrase(api_key, api_secret, passphrase)),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Spot,
            inst_type: "SPOT".to_string(),
        }
    }
    
    pub fn perpetual_demo(api_key: &str, api_secret: &str, passphrase: &str) -> Self {
        Self {
            rest: OkxRestClient::demo(ApiCredentials::with_passphrase(api_key, api_secret, passphrase)),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Perpetual,
            inst_type: "SWAP".to_string(),
        }
    }
    
    pub fn spot_public() -> Self {
        Self {
            rest: OkxRestClient::public(),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Spot,
            inst_type: "SPOT".to_string(),
        }
    }
    
    pub fn perpetual_public() -> Self {
        Self {
            rest: OkxRestClient::public(),
            ws: Arc::new(RwLock::new(None)),
            exchange_type: ExchangeType::Perpetual,
            inst_type: "SWAP".to_string(),
        }
    }
    
    fn to_inst_id(&self, pair: &str) -> String {
        let base_pair = pair.replace("/", "-");
        match self.exchange_type {
            ExchangeType::Spot => base_pair,
            ExchangeType::Perpetual | ExchangeType::SpotAndPerpetual => format!("{}-SWAP", base_pair),
        }
    }
    
    fn from_inst_id(&self, inst_id: &str) -> String {
        let without_swap = inst_id.trim_end_matches("-SWAP");
        without_swap.replace("-", "/")
    }
    
    async fn get_ws(&self) -> Result<OkxWsClient, ConnectorError> {
        let ws_read = self.ws.read().await;
        if let Some(ws) = ws_read.as_ref() {
            return Ok(ws.clone());
        }
        drop(ws_read);
        
        let mut ws_write = self.ws.write().await;
        if ws_write.is_none() {
            *ws_write = Some(OkxWsClient::new());
        }
        
        Ok(ws_write.as_ref().unwrap().clone())
    }
}

#[async_trait]
impl ConnectorBase for OkxConnector {
    fn name(&self) -> &str {
        "okx"
    }
    
    fn exchange_type(&self) -> ExchangeType {
        self.exchange_type
    }
    
    async fn get_ticker(&self, pair: &str) -> Result<Ticker, ConnectorError> {
        let inst_id = self.to_inst_id(pair);
        self.rest.get_ticker(&inst_id, pair).await
    }
    
    async fn get_orderbook(&self, pair: &str, depth: u32) -> Result<OrderBook, ConnectorError> {
        let inst_id = self.to_inst_id(pair);
        self.rest.get_orderbook(&inst_id, pair, depth).await
    }
    
    async fn get_trades(&self, pair: &str, limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        let inst_id = self.to_inst_id(pair);
        self.rest.get_trades(&inst_id, pair, limit).await
    }
    
    async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        let inst_id = self.to_inst_id(&order.pair);
        self.rest.place_order(&inst_id, &self.inst_type, order).await
    }
    
    async fn cancel_order(&self, pair: &str, order_id: &str) -> Result<CancelResponse, ConnectorError> {
        let inst_id = self.to_inst_id(pair);
        self.rest.cancel_order(&inst_id, order_id).await
    }
    
    async fn get_order(&self, pair: &str, order_id: &str) -> Result<Order, ConnectorError> {
        let inst_id = self.to_inst_id(pair);
        self.rest.get_order(&inst_id, pair, order_id).await
    }
    
    async fn get_open_orders(&self, pair: Option<&str>) -> Result<Vec<Order>, ConnectorError> {
        let inst_type = self.inst_type.clone();
        let inst_id = pair.map(|p| self.to_inst_id(p));
        self.rest.get_open_orders(&inst_type, inst_id.as_deref(), |id| {
            let without_swap = id.trim_end_matches("-SWAP");
            without_swap.replace("-", "/")
        }).await
    }
    
    async fn cancel_all_orders(&self, pair: Option<&str>) -> Result<u32, ConnectorError> {
        let orders = self.get_open_orders(pair).await?;
        let count = orders.len();
        
        for order in orders {
            let _ = self.cancel_order(&order.pair, &order.order_id).await;
        }
        
        Ok(count as u32)
    }
    
    async fn get_balance(&self, asset: &str) -> Result<Balance, ConnectorError> {
        self.rest.get_balance(asset).await
    }
    
    async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        self.rest.get_balances().await
    }
    
    async fn get_positions(&self, pair: Option<&str>) -> Result<Vec<Position>, ConnectorError> {
        let inst_type = self.inst_type.clone();
        let inst_id = pair.map(|p| self.to_inst_id(p));
        self.rest.get_positions(&inst_type, inst_id.as_deref(), |id| {
            let without_swap = id.trim_end_matches("-SWAP");
            without_swap.replace("-", "/")
        }).await
    }
    
    async fn subscribe_ticker(&self, pair: &str) -> Result<TickerStream, ConnectorError> {
        let ws = self.get_ws().await?;
        let inst_id = self.to_inst_id(pair);
        ws.subscribe_ticker(&inst_id, pair).await
    }
    
    async fn subscribe_orderbook(&self, pair: &str) -> Result<OrderBookStream, ConnectorError> {
        let ws = self.get_ws().await?;
        let inst_id = self.to_inst_id(pair);
        ws.subscribe_orderbook(&inst_id, pair).await
    }
    
    async fn subscribe_trades(&self, pair: &str) -> Result<TradeStream, ConnectorError> {
        let ws = self.get_ws().await?;
        let inst_id = self.to_inst_id(pair);
        ws.subscribe_trades(&inst_id, pair).await
    }
    
    async fn subscribe_user_data(&self) -> Result<UserDataStream, ConnectorError> {
        let ws = self.get_ws().await?;
        let credentials = self.rest.get_credentials()
            .ok_or_else(|| ConnectorError::Authentication("Credentials required for user data stream".to_string()))?;
        ws.subscribe_user_data(&credentials, |id| {
            let without_swap = id.trim_end_matches("-SWAP");
            without_swap.replace("-", "/")
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connector_creation() {
        let spot = OkxConnector::spot_public();
        assert_eq!(spot.name(), "okx");
        assert_eq!(spot.exchange_type(), ExchangeType::Spot);
        
        let perp = OkxConnector::perpetual_public();
        assert_eq!(perp.name(), "okx");
        assert_eq!(perp.exchange_type(), ExchangeType::Perpetual);
    }
    
    #[test]
    fn test_inst_id_conversion() {
        let spot = OkxConnector::spot_public();
        assert_eq!(spot.to_inst_id("BTC/USDT"), "BTC-USDT");
        
        let perp = OkxConnector::perpetual_public();
        assert_eq!(perp.to_inst_id("BTC/USDT"), "BTC-USDT-SWAP");
    }
}
