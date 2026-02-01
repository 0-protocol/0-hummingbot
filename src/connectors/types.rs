//! Common types for exchange connectors
//!
//! These types provide a unified representation for trading data
//! across different exchanges.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Ticker data for a trading pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
    /// Trading pair symbol
    pub pair: String,
    /// Last traded price
    pub last_price: Decimal,
    /// Best bid price
    pub bid: Decimal,
    /// Best ask price
    pub ask: Decimal,
    /// 24h high
    pub high_24h: Decimal,
    /// 24h low
    pub low_24h: Decimal,
    /// 24h volume (base currency)
    pub volume_24h: Decimal,
    /// 24h price change percentage
    pub change_24h: Decimal,
    /// Timestamp (unix ms)
    pub timestamp: i64,
}

/// Order book level (price and quantity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: Decimal,
    pub quantity: Decimal,
}

/// Order book snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub pair: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: i64,
}

impl OrderBook {
    /// Get the best bid price
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.first()
    }

    /// Get the best ask price
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.first()
    }

    /// Get the mid price
    pub fn mid_price(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid.price + ask.price) / Decimal::TWO),
            _ => None,
        }
    }

    /// Get the spread
    pub fn spread(&self) -> Option<Decimal> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask.price - bid.price),
            _ => None,
        }
    }
}

/// Trade data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub pair: String,
    pub price: Decimal,
    pub quantity: Decimal,
    pub side: OrderSide,
    pub timestamp: i64,
}

/// Trading pair information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPair {
    /// Symbol (e.g., "BTC-USD", "ETH-USDT")
    pub symbol: String,
    /// Base asset (e.g., "BTC", "ETH")
    pub base: String,
    /// Quote asset (e.g., "USD", "USDT")
    pub quote: String,
    /// Minimum order size
    pub min_order_size: Decimal,
    /// Tick size (minimum price increment)
    pub tick_size: Decimal,
    /// Step size (minimum quantity increment)
    pub step_size: Decimal,
    /// Whether trading is enabled
    pub is_active: bool,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "buy"),
            OrderSide::Sell => write!(f, "sell"),
        }
    }
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    /// Market order
    Market,
    /// Limit order
    Limit,
    /// Stop market order
    StopMarket,
    /// Stop limit order
    StopLimit,
    /// Take profit order
    TakeProfit,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Market => write!(f, "market"),
            OrderType::Limit => write!(f, "limit"),
            OrderType::StopMarket => write!(f, "stop_market"),
            OrderType::StopLimit => write!(f, "stop_limit"),
            OrderType::TakeProfit => write!(f, "take_profit"),
        }
    }
}

/// Time in force for orders
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good till canceled
    GTC,
    /// Immediate or cancel
    IOC,
    /// Fill or kill
    FOK,
    /// Good till date
    GTD,
    /// Post only (maker only)
    PostOnly,
}

/// Order request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    /// Trading pair
    pub pair: String,
    /// Order side
    pub side: OrderSide,
    /// Order type
    pub order_type: OrderType,
    /// Quantity
    pub quantity: Decimal,
    /// Price (required for limit orders)
    pub price: Option<Decimal>,
    /// Stop price (for stop orders)
    pub stop_price: Option<Decimal>,
    /// Time in force
    pub time_in_force: TimeInForce,
    /// Reduce only (for positions)
    pub reduce_only: bool,
    /// Post only (maker only)
    pub post_only: bool,
    /// Client order ID
    pub client_order_id: Option<String>,
}

impl OrderRequest {
    /// Create a new market order
    pub fn market(pair: &str, side: OrderSide, quantity: Decimal) -> Self {
        Self {
            pair: pair.to_string(),
            side,
            order_type: OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            time_in_force: TimeInForce::IOC,
            reduce_only: false,
            post_only: false,
            client_order_id: None,
        }
    }

    /// Create a new limit order
    pub fn limit(pair: &str, side: OrderSide, quantity: Decimal, price: Decimal) -> Self {
        Self {
            pair: pair.to_string(),
            side,
            order_type: OrderType::Limit,
            quantity,
            price: Some(price),
            stop_price: None,
            time_in_force: TimeInForce::GTC,
            reduce_only: false,
            post_only: false,
            client_order_id: None,
        }
    }
}

/// Order response from exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    /// Exchange order ID
    pub order_id: String,
    /// Client order ID (if provided)
    pub client_order_id: Option<String>,
    /// Order status
    pub status: OrderStatus,
    /// Filled quantity
    pub filled_quantity: Decimal,
    /// Average fill price
    pub avg_fill_price: Option<Decimal>,
    /// Transaction hash (for DEX orders)
    pub tx_hash: Option<String>,
    /// Timestamp
    pub timestamp: i64,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    /// Order is pending
    Pending,
    /// Order is open (partially filled or unfilled)
    Open,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is fully filled
    Filled,
    /// Order is canceled
    Canceled,
    /// Order is rejected
    Rejected,
    /// Order expired
    Expired,
}

/// Full order information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: String,
    pub client_order_id: Option<String>,
    pub pair: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub status: OrderStatus,
    pub filled_quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub avg_fill_price: Option<Decimal>,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Account balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    /// Asset symbol
    pub asset: String,
    /// Free (available) balance
    pub free: Decimal,
    /// Locked (in orders) balance
    pub locked: Decimal,
    /// Total balance
    pub total: Decimal,
}

/// Position information (for futures/perpetuals)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Trading pair
    pub pair: String,
    /// Position side
    pub side: PositionSide,
    /// Position size
    pub size: Decimal,
    /// Entry price
    pub entry_price: Decimal,
    /// Mark price
    pub mark_price: Decimal,
    /// Liquidation price
    pub liquidation_price: Option<Decimal>,
    /// Unrealized PnL
    pub unrealized_pnl: Decimal,
    /// Realized PnL
    pub realized_pnl: Decimal,
    /// Leverage
    pub leverage: Decimal,
    /// Margin used
    pub margin: Decimal,
}

/// Position side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Long,
    Short,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "long"),
            PositionSide::Short => write!(f, "short"),
        }
    }
}

/// Transaction receipt (for on-chain transactions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxReceipt {
    /// Transaction hash
    pub tx_hash: String,
    /// Block number
    pub block_number: Option<u64>,
    /// Transaction status
    pub status: TxStatus,
    /// Gas used
    pub gas_used: Option<u64>,
    /// Effective gas price
    pub gas_price: Option<Decimal>,
    /// Transaction fee
    pub fee: Option<Decimal>,
    /// Timestamp
    pub timestamp: Option<i64>,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxStatus {
    /// Transaction is pending
    Pending,
    /// Transaction confirmed successfully
    Confirmed,
    /// Transaction failed
    Failed,
}

// =========================================================================
// Additional Types for CEX Connectors
// =========================================================================

/// Cancel order response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResponse {
    /// Order ID that was canceled
    pub order_id: String,
    /// Whether cancellation was successful
    pub success: bool,
}

/// Margin mode for perpetual positions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarginMode {
    /// Cross margin
    Cross,
    /// Isolated margin
    Isolated,
}

/// User data event from WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserDataEvent {
    /// Order update
    OrderUpdate(Order),
    /// Balance update
    BalanceUpdate(Balance),
    /// Position update
    PositionUpdate(Position),
    /// Account update
    AccountUpdate { timestamp: i64 },
}

// =========================================================================
// Stream Type Aliases for WebSocket Subscriptions
// =========================================================================

use crate::connectors::ConnectorError;
use std::pin::Pin;
use futures_util::Stream;

/// Stream of ticker updates
pub type TickerStream = Pin<Box<dyn Stream<Item = Result<Ticker, ConnectorError>> + Send>>;

/// Stream of order book updates
pub type OrderBookStream = Pin<Box<dyn Stream<Item = Result<OrderBook, ConnectorError>> + Send>>;

/// Stream of trade updates
pub type TradeStream = Pin<Box<dyn Stream<Item = Result<Trade, ConnectorError>> + Send>>;

/// Stream of user data events
pub type UserDataStream = Pin<Box<dyn Stream<Item = Result<UserDataEvent, ConnectorError>> + Send>>;
