//! Binance WebSocket Client
//!
//! Real-time data streams for Binance Spot and Futures.

use async_stream::stream;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use tracing::{debug, info};

use crate::connectors::{
    error::ConnectorError,
    pair_to_symbol, symbol_to_pair,
    types::*,
    websocket::{spawn_websocket, WebSocketConfig},
};

/// Binance WebSocket client for real-time data
#[derive(Clone)]
pub struct BinanceWsClient {
    /// Base WebSocket URL
    ws_url: String,
    /// Whether this is a futures client
    is_perpetual: bool,
}

impl BinanceWsClient {
    pub fn spot() -> Self {
        Self {
            ws_url: "wss://stream.binance.com:9443/ws".to_string(),
            is_perpetual: false,
        }
    }
    
    pub fn perpetual() -> Self {
        Self {
            ws_url: "wss://fstream.binance.com/ws".to_string(),
            is_perpetual: true,
        }
    }
    
    pub fn spot_testnet() -> Self {
        Self {
            ws_url: "wss://testnet.binance.vision/ws".to_string(),
            is_perpetual: false,
        }
    }
    
    pub fn perpetual_testnet() -> Self {
        Self {
            ws_url: "wss://stream.binancefuture.com/ws".to_string(),
            is_perpetual: true,
        }
    }
    
    pub async fn subscribe_ticker(&self, pair: &str) -> Result<TickerStream, ConnectorError> {
        let symbol = pair_to_symbol(pair).to_lowercase();
        let stream_name = format!("{}@ticker", symbol);
        let url = format!("{}/{}", self.ws_url, stream_name);
        
        info!("Subscribing to ticker: {}", url);
        
        let config = WebSocketConfig::new(&url);
        let handle = spawn_websocket(config).await?;
        
        let pair_owned = pair.to_string();
        let stream = stream! {
            let mut receiver = handle.receiver;
            
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(msg) => {
                        match serde_json::from_str::<BinanceWsTickerEvent>(&msg) {
                            Ok(event) => {
                                yield Ok(Ticker {
                                    pair: pair_owned.clone(),
                                    last_price: Decimal::from_str(&event.c).unwrap_or_default(),
                                    bid: Decimal::from_str(&event.b).unwrap_or_default(),
                                    ask: Decimal::from_str(&event.a).unwrap_or_default(),
                                    high_24h: Decimal::from_str(&event.h).unwrap_or_default(),
                                    low_24h: Decimal::from_str(&event.l).unwrap_or_default(),
                                    volume_24h: Decimal::from_str(&event.v).unwrap_or_default(),
                                    change_24h: Decimal::from_str(&event.P).unwrap_or_default(),
                                    timestamp: event.E,
                                });
                            }
                            Err(e) => {
                                debug!("Failed to parse ticker event: {}", e);
                            }
                        }
                    }
                    Err(e) => yield Err(e),
                }
            }
        };
        
        Ok(Box::pin(stream))
    }
    
    pub async fn subscribe_orderbook(&self, pair: &str) -> Result<OrderBookStream, ConnectorError> {
        let symbol = pair_to_symbol(pair).to_lowercase();
        let stream_name = format!("{}@depth@100ms", symbol);
        let url = format!("{}/{}", self.ws_url, stream_name);
        
        info!("Subscribing to orderbook: {}", url);
        
        let config = WebSocketConfig::new(&url);
        let handle = spawn_websocket(config).await?;
        
        let pair_owned = pair.to_string();
        let stream = stream! {
            let mut receiver = handle.receiver;
            
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(msg) => {
                        match serde_json::from_str::<BinanceWsDepthEvent>(&msg) {
                            Ok(event) => {
                                let bids: Vec<OrderBookLevel> = event.b.iter()
                                    .map(|[price, qty]| OrderBookLevel {
                                        price: Decimal::from_str(price).unwrap_or_default(),
                                        quantity: Decimal::from_str(qty).unwrap_or_default(),
                                    })
                                    .collect();
                                
                                let asks: Vec<OrderBookLevel> = event.a.iter()
                                    .map(|[price, qty]| OrderBookLevel {
                                        price: Decimal::from_str(price).unwrap_or_default(),
                                        quantity: Decimal::from_str(qty).unwrap_or_default(),
                                    })
                                    .collect();
                                
                                yield Ok(OrderBook {
                                    pair: pair_owned.clone(),
                                    bids,
                                    asks,
                                    timestamp: event.E,
                                });
                            }
                            Err(e) => {
                                debug!("Failed to parse depth event: {}", e);
                            }
                        }
                    }
                    Err(e) => yield Err(e),
                }
            }
        };
        
        Ok(Box::pin(stream))
    }
    
    pub async fn subscribe_trades(&self, pair: &str) -> Result<TradeStream, ConnectorError> {
        let symbol = pair_to_symbol(pair).to_lowercase();
        let stream_name = format!("{}@trade", symbol);
        let url = format!("{}/{}", self.ws_url, stream_name);
        
        info!("Subscribing to trades: {}", url);
        
        let config = WebSocketConfig::new(&url);
        let handle = spawn_websocket(config).await?;
        
        let pair_owned = pair.to_string();
        let stream = stream! {
            let mut receiver = handle.receiver;
            
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(msg) => {
                        match serde_json::from_str::<BinanceWsTradeEvent>(&msg) {
                            Ok(event) => {
                                yield Ok(Trade {
                                    id: event.t.to_string(),
                                    pair: pair_owned.clone(),
                                    price: Decimal::from_str(&event.p).unwrap_or_default(),
                                    quantity: Decimal::from_str(&event.q).unwrap_or_default(),
                                    side: if event.m { OrderSide::Sell } else { OrderSide::Buy },
                                    timestamp: event.T,
                                });
                            }
                            Err(e) => {
                                debug!("Failed to parse trade event: {}", e);
                            }
                        }
                    }
                    Err(e) => yield Err(e),
                }
            }
        };
        
        Ok(Box::pin(stream))
    }
    
    pub async fn subscribe_user_data(&self, listen_key: &str) -> Result<UserDataStream, ConnectorError> {
        let url = format!("{}/{}", self.ws_url, listen_key);
        
        info!("Subscribing to user data stream");
        
        let config = WebSocketConfig::new(&url);
        let handle = spawn_websocket(config).await?;
        
        let is_perpetual = self.is_perpetual;
        let stream = stream! {
            let mut receiver = handle.receiver;
            
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(msg) => {
                        if let Ok(event) = serde_json::from_str::<BinanceWsUserEvent>(&msg) {
                            match event.e.as_str() {
                                "executionReport" => {
                                    if let Ok(order_event) = serde_json::from_str::<BinanceWsOrderUpdateEvent>(&msg) {
                                        let filled = Decimal::from_str(&order_event.z).unwrap_or_default();
                                        let orig = Decimal::from_str(&order_event.q).unwrap_or_default();
                                        
                                        yield Ok(UserDataEvent::OrderUpdate(Order {
                                            order_id: order_event.i.to_string(),
                                            client_order_id: Some(order_event.c.clone()),
                                            pair: symbol_to_pair(&order_event.s, 4),
                                            side: if order_event.S == "BUY" { OrderSide::Buy } else { OrderSide::Sell },
                                            order_type: parse_ws_order_type(&order_event.o),
                                            quantity: orig,
                                            price: Decimal::from_str(&order_event.p).ok(),
                                            stop_price: None,
                                            status: parse_ws_order_status(&order_event.X),
                                            filled_quantity: filled,
                                            remaining_quantity: orig - filled,
                                            avg_fill_price: Decimal::from_str(&order_event.ap).ok(),
                                            time_in_force: TimeInForce::GTC,
                                            reduce_only: false,
                                            created_at: order_event.O,
                                            updated_at: order_event.E,
                                        }));
                                    }
                                }
                                "outboundAccountPosition" => {
                                    if let Ok(balance_event) = serde_json::from_str::<BinanceWsBalanceUpdateEvent>(&msg) {
                                        for b in balance_event.B {
                                            let free = Decimal::from_str(&b.f).unwrap_or_default();
                                            let locked = Decimal::from_str(&b.l).unwrap_or_default();
                                            yield Ok(UserDataEvent::BalanceUpdate(Balance {
                                                asset: b.a,
                                                free,
                                                locked,
                                                total: free + locked,
                                            }));
                                        }
                                    }
                                }
                                "ACCOUNT_UPDATE" if is_perpetual => {
                                    if let Ok(account_event) = serde_json::from_str::<BinanceWsFuturesAccountEvent>(&msg) {
                                        for b in account_event.a.B {
                                            let free = Decimal::from_str(&b.cw).unwrap_or_default();
                                            yield Ok(UserDataEvent::BalanceUpdate(Balance {
                                                asset: b.a,
                                                free,
                                                locked: Decimal::ZERO,
                                                total: free,
                                            }));
                                        }
                                        
                                        for p in account_event.a.P {
                                            let qty = Decimal::from_str(&p.pa).unwrap_or_default();
                                            if !qty.is_zero() {
                                                yield Ok(UserDataEvent::PositionUpdate(Position {
                                                    pair: symbol_to_pair(&p.s, 4),
                                                    side: if qty > Decimal::ZERO { PositionSide::Long } else { PositionSide::Short },
                                                    size: qty.abs(),
                                                    entry_price: Decimal::from_str(&p.ep).unwrap_or_default(),
                                                    mark_price: Decimal::ZERO,
                                                    liquidation_price: None,
                                                    unrealized_pnl: Decimal::from_str(&p.up).unwrap_or_default(),
                                                    realized_pnl: Decimal::ZERO,
                                                    leverage: Decimal::ONE,
                                                    margin: Decimal::ZERO,
                                                }));
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    debug!("Unknown user data event type: {}", event.e);
                                }
                            }
                        }
                    }
                    Err(e) => yield Err(e),
                }
            }
        };
        
        Ok(Box::pin(stream))
    }
}

// =========================================================================
// WebSocket Event Types
// =========================================================================

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct BinanceWsTickerEvent {
    E: i64,         // Event time
    c: String,      // Last price
    h: String,      // High price
    l: String,      // Low price
    v: String,      // Total traded volume
    b: String,      // Best bid price
    a: String,      // Best ask price
    P: String,      // Price change percent
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct BinanceWsDepthEvent {
    E: i64,         // Event time
    b: Vec<[String; 2]>,  // Bids
    a: Vec<[String; 2]>,  // Asks
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct BinanceWsTradeEvent {
    t: u64,         // Trade ID
    p: String,      // Price
    q: String,      // Quantity
    T: i64,         // Trade time
    m: bool,        // Is buyer maker
}

#[derive(Debug, Deserialize)]
struct BinanceWsUserEvent {
    e: String,      // Event type
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct BinanceWsOrderUpdateEvent {
    E: i64,         // Event time
    s: String,      // Symbol
    c: String,      // Client order ID
    S: String,      // Side
    o: String,      // Order type
    q: String,      // Order quantity
    p: String,      // Order price
    X: String,      // Current order status
    i: u64,         // Order ID
    z: String,      // Cumulative filled quantity
    ap: String,     // Average price
    O: i64,         // Order creation time
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct BinanceWsBalanceUpdateEvent {
    B: Vec<BinanceWsBalance>,
}

#[derive(Debug, Deserialize)]
struct BinanceWsBalance {
    a: String,      // Asset
    f: String,      // Free
    l: String,      // Locked
}

#[derive(Debug, Deserialize)]
struct BinanceWsFuturesAccountEvent {
    a: BinanceWsFuturesAccountData,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct BinanceWsFuturesAccountData {
    B: Vec<BinanceWsFuturesBalance>,
    P: Vec<BinanceWsFuturesPosition>,
}

#[derive(Debug, Deserialize)]
struct BinanceWsFuturesBalance {
    a: String,      // Asset
    cw: String,     // Cross wallet balance
}

#[derive(Debug, Deserialize)]
struct BinanceWsFuturesPosition {
    s: String,      // Symbol
    pa: String,     // Position amount
    ep: String,     // Entry price
    up: String,     // Unrealized PnL
}

// =========================================================================
// Helper Functions
// =========================================================================

fn parse_ws_order_status(status: &str) -> OrderStatus {
    match status {
        "NEW" => OrderStatus::Open,
        "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
        "FILLED" => OrderStatus::Filled,
        "CANCELED" => OrderStatus::Canceled,
        "REJECTED" => OrderStatus::Rejected,
        "EXPIRED" => OrderStatus::Expired,
        _ => OrderStatus::Pending,
    }
}

fn parse_ws_order_type(order_type: &str) -> OrderType {
    match order_type {
        "MARKET" => OrderType::Market,
        "LIMIT" => OrderType::Limit,
        "STOP_LOSS_LIMIT" => OrderType::StopLimit,
        "STOP_MARKET" => OrderType::StopMarket,
        "TAKE_PROFIT" | "TAKE_PROFIT_LIMIT" => OrderType::TakeProfit,
        _ => OrderType::Limit,
    }
}
