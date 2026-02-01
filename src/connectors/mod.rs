//! Exchange Connectors for 0-hummingbot
//!
//! This module provides a unified interface for interacting with cryptocurrency exchanges.
//! All connectors implement the `ConnectorBase` trait, providing a consistent API for:
//! - Market data retrieval (ticker, orderbook, trades)
//! - Order management (place, cancel, query)
//! - Account management (balances, positions)
//! - Real-time data streams via WebSocket

pub mod auth;
pub mod cex;
pub mod error;
pub mod types;
pub mod websocket;

// Re-export commonly used items
pub use auth::{ApiCredentials, hmac_sha256_sign, hmac_sha256_sign_base64};
pub use error::ConnectorError;
pub use types::*;

use async_trait::async_trait;

/// Base trait for all exchange connectors
#[async_trait]
pub trait ConnectorBase: Send + Sync {
    /// Get the exchange name
    fn name(&self) -> &str;
    
    /// Get the exchange type (spot, perpetual, etc.)
    fn exchange_type(&self) -> ExchangeType;
    
    // =========================================================================
    // Market Data (REST)
    // =========================================================================
    
    /// Get ticker data for a trading pair
    async fn get_ticker(&self, pair: &str) -> Result<Ticker, ConnectorError>;
    
    /// Get order book for a trading pair
    async fn get_orderbook(&self, pair: &str, depth: u32) -> Result<OrderBook, ConnectorError>;
    
    /// Get recent trades for a trading pair
    async fn get_trades(&self, pair: &str, limit: u32) -> Result<Vec<Trade>, ConnectorError>;
    
    // =========================================================================
    // Trading (REST)
    // =========================================================================
    
    /// Place a new order
    async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse, ConnectorError>;
    
    /// Cancel an existing order
    async fn cancel_order(&self, pair: &str, order_id: &str) -> Result<CancelResponse, ConnectorError>;
    
    /// Get details of a specific order
    async fn get_order(&self, pair: &str, order_id: &str) -> Result<Order, ConnectorError>;
    
    /// Get all open orders
    async fn get_open_orders(&self, pair: Option<&str>) -> Result<Vec<Order>, ConnectorError>;
    
    /// Cancel all open orders
    async fn cancel_all_orders(&self, pair: Option<&str>) -> Result<u32, ConnectorError>;
    
    // =========================================================================
    // Account (REST)
    // =========================================================================
    
    /// Get balance for a specific asset
    async fn get_balance(&self, asset: &str) -> Result<Balance, ConnectorError>;
    
    /// Get all balances
    async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError>;
    
    /// Get positions (for perpetual contracts)
    async fn get_positions(&self, pair: Option<&str>) -> Result<Vec<Position>, ConnectorError>;
    
    // =========================================================================
    // WebSocket Subscriptions
    // =========================================================================
    
    /// Subscribe to ticker updates
    async fn subscribe_ticker(&self, pair: &str) -> Result<TickerStream, ConnectorError>;
    
    /// Subscribe to order book updates
    async fn subscribe_orderbook(&self, pair: &str) -> Result<OrderBookStream, ConnectorError>;
    
    /// Subscribe to trade updates
    async fn subscribe_trades(&self, pair: &str) -> Result<TradeStream, ConnectorError>;
    
    /// Subscribe to user data (orders, balances, positions)
    async fn subscribe_user_data(&self) -> Result<UserDataStream, ConnectorError>;
}

/// Exchange type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeType {
    /// Spot trading
    Spot,
    /// Perpetual futures
    Perpetual,
    /// Both spot and perpetual
    SpotAndPerpetual,
}

impl std::fmt::Display for ExchangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExchangeType::Spot => write!(f, "spot"),
            ExchangeType::Perpetual => write!(f, "perpetual"),
            ExchangeType::SpotAndPerpetual => write!(f, "spot+perpetual"),
        }
    }
}

/// Convert trading pair format: "BTC/USDT" -> "BTCUSDT"
pub fn pair_to_symbol(pair: &str) -> String {
    pair.replace("/", "")
}

/// Convert symbol to pair format: "BTCUSDT" -> "BTC/USDT"
pub fn symbol_to_pair(symbol: &str, quote_len: usize) -> String {
    if symbol.len() > quote_len {
        let (base, quote) = symbol.split_at(symbol.len() - quote_len);
        format!("{}/{}", base, quote)
    } else {
        symbol.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pair_to_symbol() {
        assert_eq!(pair_to_symbol("BTC/USDT"), "BTCUSDT");
        assert_eq!(pair_to_symbol("ETH/BTC"), "ETHBTC");
    }
    
    #[test]
    fn test_symbol_to_pair() {
        assert_eq!(symbol_to_pair("BTCUSDT", 4), "BTC/USDT");
        assert_eq!(symbol_to_pair("ETHBTC", 3), "ETH/BTC");
    }
}
