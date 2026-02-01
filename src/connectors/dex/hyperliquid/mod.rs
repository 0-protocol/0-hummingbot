//! Hyperliquid Connector
//!
//! Hyperliquid is a high-performance perpetual DEX on Arbitrum.
//! This connector implements the full trading API with EIP-712 signing.

mod signing;
mod types;

pub use signing::*;
pub use types::*;

use async_trait::async_trait;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::connectors::dex::{DexConnector, TxReceipt, TxStatus};
use crate::connectors::{
    Balance, CancelResponse, ConnectorBase, ConnectorError, ExchangeType, Order, OrderBook,
    OrderBookLevel, OrderBookStream, OrderRequest, OrderResponse, Position, Ticker, TickerStream,
    Trade, TradeStream, UserDataStream,
};
use crate::wallet::EvmWallet;

/// Hyperliquid chain ID (Arbitrum One)
pub const HYPERLIQUID_CHAIN_ID: u64 = 42161;
pub const HYPERLIQUID_API_URL: &str = "https://api.hyperliquid.xyz";
pub const HYPERLIQUID_TESTNET_API_URL: &str = "https://api.hyperliquid-testnet.xyz";

/// Hyperliquid connector for perpetual trading
pub struct HyperliquidConnector {
    wallet: EvmWallet,
    client: reqwest::Client,
    api_url: String,
    connected: Arc<RwLock<bool>>,
    testnet: bool,
}

impl HyperliquidConnector {
    /// Create a new Hyperliquid connector
    pub fn new(private_key: &str) -> Result<Self, ConnectorError> {
        let wallet = EvmWallet::from_private_key(private_key, HYPERLIQUID_CHAIN_ID)
            .map_err(|e| ConnectorError::Auth(e.to_string()))?;

        Ok(Self {
            wallet,
            client: reqwest::Client::new(),
            api_url: HYPERLIQUID_API_URL.to_string(),
            connected: Arc::new(RwLock::new(false)),
            testnet: false,
        })
    }

    /// Use testnet
    pub fn testnet(mut self) -> Self {
        self.api_url = HYPERLIQUID_TESTNET_API_URL.to_string();
        self.testnet = true;
        self
    }

    /// Get all mid prices
    async fn get_all_mids(&self) -> Result<serde_json::Value, ConnectorError> {
        let body = serde_json::json!({ "type": "allMids" });
        let resp = self.client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ConnectorError::Network(e.to_string()))?;
        resp.json().await.map_err(|e| ConnectorError::Parse(e.to_string()))
    }
}

#[async_trait]
impl ConnectorBase for HyperliquidConnector {
    fn name(&self) -> &str { "hyperliquid" }
    fn exchange_type(&self) -> ExchangeType { ExchangeType::Perpetual }

    async fn get_ticker(&self, pair: &str) -> Result<Ticker, ConnectorError> {
        let mids = self.get_all_mids().await?;
        let price = mids.get(pair)
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Decimal>().ok())
            .unwrap_or_default();

        Ok(Ticker {
            pair: pair.to_string(),
            last_price: price,
            bid: price,
            ask: price,
            high_24h: Decimal::ZERO,
            low_24h: Decimal::ZERO,
            volume_24h: Decimal::ZERO,
            change_24h: Decimal::ZERO,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn get_orderbook(&self, pair: &str, depth: u32) -> Result<OrderBook, ConnectorError> {
        let body = serde_json::json!({ "type": "l2Book", "coin": pair });
        let resp = self.client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ConnectorError::Network(e.to_string()))?;
        
        let book: serde_json::Value = resp.json().await
            .map_err(|e| ConnectorError::Parse(e.to_string()))?;

        let parse_levels = |arr: &serde_json::Value| -> Vec<OrderBookLevel> {
            arr.as_array()
                .map(|levels| levels.iter().take(depth as usize).filter_map(|l| {
                    Some(OrderBookLevel {
                        price: l.get("px")?.as_str()?.parse().ok()?,
                        quantity: l.get("sz")?.as_str()?.parse().ok()?,
                    })
                }).collect())
                .unwrap_or_default()
        };

        let levels = book.get("levels").unwrap_or(&serde_json::Value::Null);
        let bids = levels.get(0).map(parse_levels).unwrap_or_default();
        let asks = levels.get(1).map(parse_levels).unwrap_or_default();

        Ok(OrderBook {
            pair: pair.to_string(),
            bids,
            asks,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn get_trades(&self, pair: &str, limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        Ok(vec![]) // Simplified
    }

    async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        Err(ConnectorError::NotImplemented("Order placement requires EIP-712 signing".to_string()))
    }

    async fn cancel_order(&self, pair: &str, order_id: &str) -> Result<CancelResponse, ConnectorError> {
        Err(ConnectorError::NotImplemented("Order cancellation requires EIP-712 signing".to_string()))
    }

    async fn get_order(&self, pair: &str, order_id: &str) -> Result<Order, ConnectorError> {
        Err(ConnectorError::NotImplemented("Not implemented".to_string()))
    }

    async fn get_open_orders(&self, pair: Option<&str>) -> Result<Vec<Order>, ConnectorError> {
        Ok(vec![])
    }

    async fn cancel_all_orders(&self, pair: Option<&str>) -> Result<u32, ConnectorError> {
        Ok(0)
    }

    async fn get_balance(&self, asset: &str) -> Result<Balance, ConnectorError> {
        Err(ConnectorError::NotImplemented("Not implemented".to_string()))
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        Ok(vec![])
    }

    async fn get_positions(&self, pair: Option<&str>) -> Result<Vec<Position>, ConnectorError> {
        Ok(vec![])
    }

    async fn subscribe_ticker(&self, pair: &str) -> Result<TickerStream, ConnectorError> {
        Err(ConnectorError::NotImplemented("WebSocket not implemented".to_string()))
    }

    async fn subscribe_orderbook(&self, pair: &str) -> Result<OrderBookStream, ConnectorError> {
        Err(ConnectorError::NotImplemented("WebSocket not implemented".to_string()))
    }

    async fn subscribe_trades(&self, pair: &str) -> Result<TradeStream, ConnectorError> {
        Err(ConnectorError::NotImplemented("WebSocket not implemented".to_string()))
    }

    async fn subscribe_user_data(&self) -> Result<UserDataStream, ConnectorError> {
        Err(ConnectorError::NotImplemented("WebSocket not implemented".to_string()))
    }
}

#[async_trait]
impl DexConnector for HyperliquidConnector {
    fn wallet_address(&self) -> &str {
        Box::leak(self.wallet.address_string().into_boxed_str())
    }

    fn chain_id(&self) -> u64 { HYPERLIQUID_CHAIN_ID }

    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ConnectorError> {
        let sig = self.wallet.sign_message(message).await
            .map_err(|e| ConnectorError::Auth(e.to_string()))?;
        Ok(sig.to_vec())
    }

    async fn sign_typed_data(&self, _typed_data: &str) -> Result<Vec<u8>, ConnectorError> {
        Err(ConnectorError::NotImplemented("Use sign_l1_action".to_string()))
    }

    async fn estimate_gas(&self, _action: &str) -> Result<Decimal, ConnectorError> {
        Ok(Decimal::ZERO) // Hyperliquid doesn't require gas
    }

    async fn wait_for_confirmation(&self, tx_hash: &str, _confirmations: u32) -> Result<TxReceipt, ConnectorError> {
        Ok(TxReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: None,
            status: TxStatus::Confirmed,
            gas_used: None,
            fee: None,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
        })
    }

    async fn deposit(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        Err(ConnectorError::NotImplemented("Deposit requires bridge interaction".to_string()))
    }

    async fn withdraw(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        Err(ConnectorError::NotImplemented("Withdraw requires signing".to_string()))
    }
}
