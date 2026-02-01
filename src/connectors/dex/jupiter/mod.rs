//! Jupiter Connector
//!
//! Jupiter is the leading swap aggregator on Solana.

use async_trait::async_trait;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::connectors::dex::{DexConnector, TxReceipt, TxStatus};
use crate::connectors::{
    Balance, CancelResponse, ConnectorBase, ConnectorError, ExchangeType, Order, OrderBook,
    OrderBookStream, OrderRequest, OrderResponse, Position, Ticker, TickerStream,
    Trade, TradeStream, UserDataStream,
};
use crate::wallet::SolanaWallet;

pub const JUPITER_API_URL: &str = "https://quote-api.jup.ag/v6";
pub const JUPITER_PRICE_API_URL: &str = "https://price.jup.ag/v6";

/// Common Solana token mints
pub mod tokens {
    pub const SOL: &str = "So11111111111111111111111111111111111111112";
    pub const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
}

/// Jupiter connector for Solana swaps
pub struct JupiterConnector {
    wallet: SolanaWallet,
    client: reqwest::Client,
    api_url: String,
    price_api_url: String,
    slippage_bps: u32,
}

impl JupiterConnector {
    pub fn new(private_key: &[u8]) -> Result<Self, ConnectorError> {
        let wallet = SolanaWallet::from_private_key(private_key)
            .map_err(|e| ConnectorError::Auth(e.to_string()))?;

        Ok(Self {
            wallet,
            client: reqwest::Client::new(),
            api_url: JUPITER_API_URL.to_string(),
            price_api_url: JUPITER_PRICE_API_URL.to_string(),
            slippage_bps: 50, // 0.5% default
        })
    }

    pub fn from_base58(private_key: &str) -> Result<Self, ConnectorError> {
        let wallet = SolanaWallet::from_base58(private_key)
            .map_err(|e| ConnectorError::Auth(e.to_string()))?;

        Ok(Self {
            wallet,
            client: reqwest::Client::new(),
            api_url: JUPITER_API_URL.to_string(),
            price_api_url: JUPITER_PRICE_API_URL.to_string(),
            slippage_bps: 50,
        })
    }

    /// Get token price
    async fn get_price(&self, mint: &str) -> Result<Decimal, ConnectorError> {
        let resp = self.client
            .get(&format!("{}/price?ids={}", self.price_api_url, mint))
            .send().await
            .map_err(|e| ConnectorError::Network(e.to_string()))?;

        let data: serde_json::Value = resp.json().await
            .map_err(|e| ConnectorError::Parse(e.to_string()))?;

        data.get("data")
            .and_then(|d| d.get(mint))
            .and_then(|p| p.get("price"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ConnectorError::Parse("Price not found".to_string()))
    }
}

#[async_trait]
impl ConnectorBase for JupiterConnector {
    fn name(&self) -> &str { "jupiter" }
    fn exchange_type(&self) -> ExchangeType { ExchangeType::Spot }

    async fn get_ticker(&self, pair: &str) -> Result<Ticker, ConnectorError> {
        let price = self.get_price(tokens::SOL).await.unwrap_or_default();

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

    async fn get_orderbook(&self, pair: &str, _depth: u32) -> Result<OrderBook, ConnectorError> {
        Err(ConnectorError::NotImplemented("Jupiter is a swap aggregator - no orderbook".to_string()))
    }

    async fn get_trades(&self, _pair: &str, _limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        Ok(vec![])
    }

    async fn place_order(&self, _order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        Err(ConnectorError::NotImplemented("Use Jupiter quote/swap API".to_string()))
    }

    async fn cancel_order(&self, _pair: &str, _order_id: &str) -> Result<CancelResponse, ConnectorError> {
        Err(ConnectorError::NotImplemented("Swaps are atomic".to_string()))
    }

    async fn get_order(&self, _pair: &str, _order_id: &str) -> Result<Order, ConnectorError> {
        Err(ConnectorError::NotImplemented("Not applicable".to_string()))
    }

    async fn get_open_orders(&self, _pair: Option<&str>) -> Result<Vec<Order>, ConnectorError> {
        Ok(vec![])
    }

    async fn cancel_all_orders(&self, _pair: Option<&str>) -> Result<u32, ConnectorError> {
        Ok(0)
    }

    async fn get_balance(&self, _asset: &str) -> Result<Balance, ConnectorError> {
        Err(ConnectorError::NotImplemented("Query Solana RPC".to_string()))
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        Ok(vec![])
    }

    async fn get_positions(&self, _pair: Option<&str>) -> Result<Vec<Position>, ConnectorError> {
        Ok(vec![])
    }

    async fn subscribe_ticker(&self, _pair: &str) -> Result<TickerStream, ConnectorError> {
        Err(ConnectorError::NotImplemented("Not implemented".to_string()))
    }

    async fn subscribe_orderbook(&self, _pair: &str) -> Result<OrderBookStream, ConnectorError> {
        Err(ConnectorError::NotImplemented("Not implemented".to_string()))
    }

    async fn subscribe_trades(&self, _pair: &str) -> Result<TradeStream, ConnectorError> {
        Err(ConnectorError::NotImplemented("Not implemented".to_string()))
    }

    async fn subscribe_user_data(&self) -> Result<UserDataStream, ConnectorError> {
        Err(ConnectorError::NotImplemented("Not implemented".to_string()))
    }
}

#[async_trait]
impl DexConnector for JupiterConnector {
    fn wallet_address(&self) -> &str {
        Box::leak(self.wallet.pubkey_string().into_boxed_str())
    }

    fn chain_id(&self) -> u64 { 0 } // Solana doesn't use chain IDs

    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ConnectorError> {
        Ok(self.wallet.sign_bytes(message))
    }

    async fn sign_typed_data(&self, _typed_data: &str) -> Result<Vec<u8>, ConnectorError> {
        Err(ConnectorError::NotImplemented("Solana doesn't use EIP-712".to_string()))
    }

    async fn estimate_gas(&self, _action: &str) -> Result<Decimal, ConnectorError> {
        Ok(Decimal::new(5000, 9)) // ~5000 lamports
    }

    async fn wait_for_confirmation(&self, tx_hash: &str, _confirmations: u32) -> Result<TxReceipt, ConnectorError> {
        Ok(TxReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: None,
            status: TxStatus::Pending,
            gas_used: None,
            fee: Some(Decimal::new(5000, 9)),
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
        })
    }

    async fn deposit(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        Err(ConnectorError::NotImplemented("No deposits needed for Jupiter".to_string()))
    }

    async fn withdraw(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        Err(ConnectorError::NotImplemented("No withdrawals needed for Jupiter".to_string()))
    }
}
