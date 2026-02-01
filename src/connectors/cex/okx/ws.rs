//! OKX WebSocket Client
//!
//! Real-time data streams for OKX.

use async_stream::stream;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use tracing::info;

use crate::connectors::{
    auth::{hmac_sha256_sign_base64, timestamp_ms, ApiCredentials},
    error::ConnectorError,
    types::*,
    websocket::{spawn_websocket, subscribe, WebSocketConfig},
};

/// OKX WebSocket client for real-time data
#[derive(Clone)]
pub struct OkxWsClient {
    ws_public_url: String,
    ws_private_url: String,
}

impl OkxWsClient {
    pub fn new() -> Self {
        Self {
            ws_public_url: "wss://ws.okx.com:8443/ws/v5/public".to_string(),
            ws_private_url: "wss://ws.okx.com:8443/ws/v5/private".to_string(),
        }
    }
    
    pub fn demo() -> Self {
        Self {
            ws_public_url: "wss://wspap.okx.com:8443/ws/v5/public?brokerId=9999".to_string(),
            ws_private_url: "wss://wspap.okx.com:8443/ws/v5/private?brokerId=9999".to_string(),
        }
    }
    
    pub async fn subscribe_ticker(&self, inst_id: &str, pair: &str) -> Result<TickerStream, ConnectorError> {
        let config = WebSocketConfig::new(&self.ws_public_url);
        let handle = spawn_websocket(config).await?;
        
        let sub_msg = serde_json::json!({
            "op": "subscribe",
            "args": [{"channel": "tickers", "instId": inst_id}]
        }).to_string();
        
        subscribe(&handle.sender, &sub_msg).await?;
        
        info!("Subscribed to OKX ticker: {}", inst_id);
        
        let pair_owned = pair.to_string();
        let stream = stream! {
            let mut receiver = handle.receiver;
            
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(msg) => {
                        if let Ok(event) = serde_json::from_str::<OkxWsTickerEvent>(&msg) {
                            if event.arg.channel == "tickers" {
                                for data in event.data {
                                    yield Ok(Ticker {
                                        pair: pair_owned.clone(),
                                        last_price: Decimal::from_str(&data.last).unwrap_or_default(),
                                        bid: Decimal::from_str(&data.bid_px).unwrap_or_default(),
                                        ask: Decimal::from_str(&data.ask_px).unwrap_or_default(),
                                        high_24h: Decimal::from_str(&data.high_24h).unwrap_or_default(),
                                        low_24h: Decimal::from_str(&data.low_24h).unwrap_or_default(),
                                        volume_24h: Decimal::from_str(&data.vol_24h).unwrap_or_default(),
                                        change_24h: Decimal::ZERO,
                                        timestamp: data.ts.parse().unwrap_or(timestamp_ms() as i64),
                                    });
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
    
    pub async fn subscribe_orderbook(&self, inst_id: &str, pair: &str) -> Result<OrderBookStream, ConnectorError> {
        let config = WebSocketConfig::new(&self.ws_public_url);
        let handle = spawn_websocket(config).await?;
        
        let sub_msg = serde_json::json!({
            "op": "subscribe",
            "args": [{"channel": "books", "instId": inst_id}]
        }).to_string();
        
        subscribe(&handle.sender, &sub_msg).await?;
        
        info!("Subscribed to OKX orderbook: {}", inst_id);
        
        let pair_owned = pair.to_string();
        let stream = stream! {
            let mut receiver = handle.receiver;
            
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(msg) => {
                        if let Ok(event) = serde_json::from_str::<OkxWsOrderBookEvent>(&msg) {
                            if event.arg.channel == "books" {
                                for data in event.data {
                                    let bids: Vec<OrderBookLevel> = data.bids.iter()
                                        .map(|level| OrderBookLevel {
                                            price: Decimal::from_str(&level[0]).unwrap_or_default(),
                                            quantity: Decimal::from_str(&level[1]).unwrap_or_default(),
                                        })
                                        .collect();
                                    
                                    let asks: Vec<OrderBookLevel> = data.asks.iter()
                                        .map(|level| OrderBookLevel {
                                            price: Decimal::from_str(&level[0]).unwrap_or_default(),
                                            quantity: Decimal::from_str(&level[1]).unwrap_or_default(),
                                        })
                                        .collect();
                                    
                                    yield Ok(OrderBook {
                                        pair: pair_owned.clone(),
                                        bids,
                                        asks,
                                        timestamp: data.ts.parse().unwrap_or(timestamp_ms() as i64),
                                    });
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
    
    pub async fn subscribe_trades(&self, inst_id: &str, pair: &str) -> Result<TradeStream, ConnectorError> {
        let config = WebSocketConfig::new(&self.ws_public_url);
        let handle = spawn_websocket(config).await?;
        
        let sub_msg = serde_json::json!({
            "op": "subscribe",
            "args": [{"channel": "trades", "instId": inst_id}]
        }).to_string();
        
        subscribe(&handle.sender, &sub_msg).await?;
        
        info!("Subscribed to OKX trades: {}", inst_id);
        
        let pair_owned = pair.to_string();
        let stream = stream! {
            let mut receiver = handle.receiver;
            
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(msg) => {
                        if let Ok(event) = serde_json::from_str::<OkxWsTradeEvent>(&msg) {
                            if event.arg.channel == "trades" {
                                for data in event.data {
                                    yield Ok(Trade {
                                        id: data.trade_id,
                                        pair: pair_owned.clone(),
                                        price: Decimal::from_str(&data.px).unwrap_or_default(),
                                        quantity: Decimal::from_str(&data.sz).unwrap_or_default(),
                                        side: if data.side == "buy" { OrderSide::Buy } else { OrderSide::Sell },
                                        timestamp: data.ts.parse().unwrap_or(0),
                                    });
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
    
    pub async fn subscribe_user_data<F>(&self, credentials: &ApiCredentials, pair_converter: F) -> Result<UserDataStream, ConnectorError>
    where
        F: Fn(&str) -> String + Send + 'static,
    {
        let config = WebSocketConfig::new(&self.ws_private_url);
        let handle = spawn_websocket(config).await?;
        
        // Authenticate
        let timestamp = (timestamp_ms() / 1000).to_string();
        let sign_payload = format!("{}GET/users/self/verify", timestamp);
        let sign = hmac_sha256_sign_base64(&credentials.api_secret, &sign_payload);
        
        let login_msg = serde_json::json!({
            "op": "login",
            "args": [{
                "apiKey": credentials.api_key,
                "passphrase": credentials.passphrase,
                "timestamp": timestamp,
                "sign": sign
            }]
        }).to_string();
        
        subscribe(&handle.sender, &login_msg).await?;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        let sub_msg = serde_json::json!({
            "op": "subscribe",
            "args": [
                {"channel": "orders", "instType": "ANY"},
                {"channel": "positions", "instType": "ANY"},
                {"channel": "account"}
            ]
        }).to_string();
        
        subscribe(&handle.sender, &sub_msg).await?;
        
        info!("Subscribed to OKX user data stream");
        
        let stream = stream! {
            let mut receiver = handle.receiver;
            
            while let Some(result) = receiver.recv().await {
                match result {
                    Ok(msg) => {
                        // Order updates
                        if let Ok(event) = serde_json::from_str::<OkxWsOrderEvent>(&msg) {
                            if event.arg.channel == "orders" {
                                for data in event.data {
                                    let filled = Decimal::from_str(&data.acc_fill_sz).unwrap_or_default();
                                    let orig = Decimal::from_str(&data.sz).unwrap_or_default();
                                    
                                    yield Ok(UserDataEvent::OrderUpdate(Order {
                                        order_id: data.ord_id,
                                        client_order_id: if data.cl_ord_id.is_empty() { None } else { Some(data.cl_ord_id) },
                                        pair: pair_converter(&data.inst_id),
                                        side: if data.side == "buy" { OrderSide::Buy } else { OrderSide::Sell },
                                        order_type: parse_ws_order_type(&data.ord_type),
                                        quantity: orig,
                                        price: Decimal::from_str(&data.px).ok(),
                                        stop_price: None,
                                        status: parse_ws_order_status(&data.state),
                                        filled_quantity: filled,
                                        remaining_quantity: orig - filled,
                                        avg_fill_price: Decimal::from_str(&data.avg_px).ok(),
                                        time_in_force: TimeInForce::GTC,
                                        reduce_only: false,
                                        created_at: data.c_time.parse().unwrap_or(0),
                                        updated_at: data.u_time.parse().unwrap_or(0),
                                    }));
                                }
                            }
                        }
                        
                        // Position updates
                        if let Ok(event) = serde_json::from_str::<OkxWsPositionEvent>(&msg) {
                            if event.arg.channel == "positions" {
                                for data in event.data {
                                    let qty = Decimal::from_str(&data.pos).unwrap_or_default();
                                    if !qty.is_zero() {
                                        yield Ok(UserDataEvent::PositionUpdate(Position {
                                            pair: pair_converter(&data.inst_id),
                                            side: match data.pos_side.as_str() {
                                                "long" => PositionSide::Long,
                                                "short" => PositionSide::Short,
                                                _ => if qty > Decimal::ZERO { PositionSide::Long } else { PositionSide::Short },
                                            },
                                            size: qty.abs(),
                                            entry_price: Decimal::from_str(&data.avg_px).unwrap_or_default(),
                                            mark_price: Decimal::from_str(&data.mark_px).unwrap_or_default(),
                                            liquidation_price: Decimal::from_str(&data.liq_px).ok(),
                                            unrealized_pnl: Decimal::from_str(&data.upl).unwrap_or_default(),
                                            realized_pnl: Decimal::ZERO,
                                            leverage: Decimal::from_str(&data.lever).unwrap_or(Decimal::ONE),
                                            margin: Decimal::ZERO,
                                        }));
                                    }
                                }
                            }
                        }
                        
                        // Account updates
                        if let Ok(event) = serde_json::from_str::<OkxWsAccountEvent>(&msg) {
                            if event.arg.channel == "account" {
                                for data in event.data {
                                    for detail in data.details {
                                        let free = Decimal::from_str(&detail.avail_bal).unwrap_or_default();
                                        let locked = Decimal::from_str(&detail.frozen_bal).unwrap_or_default();
                                        yield Ok(UserDataEvent::BalanceUpdate(Balance {
                                            asset: detail.ccy,
                                            free,
                                            locked,
                                            total: free + locked,
                                        }));
                                    }
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

impl Default for OkxWsClient {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// WebSocket Event Types
// =========================================================================

#[derive(Debug, Deserialize)]
struct OkxWsArg {
    channel: String,
}

#[derive(Debug, Deserialize)]
struct OkxWsTickerEvent {
    arg: OkxWsArg,
    data: Vec<OkxWsTickerData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxWsTickerData {
    last: String,
    bid_px: String,
    ask_px: String,
    vol_24h: String,
    high_24h: String,
    low_24h: String,
    ts: String,
}

#[derive(Debug, Deserialize)]
struct OkxWsOrderBookEvent {
    arg: OkxWsArg,
    data: Vec<OkxWsOrderBookData>,
}

#[derive(Debug, Deserialize)]
struct OkxWsOrderBookData {
    asks: Vec<Vec<String>>,
    bids: Vec<Vec<String>>,
    ts: String,
}

#[derive(Debug, Deserialize)]
struct OkxWsTradeEvent {
    arg: OkxWsArg,
    data: Vec<OkxWsTradeData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxWsTradeData {
    trade_id: String,
    px: String,
    sz: String,
    side: String,
    ts: String,
}

#[derive(Debug, Deserialize)]
struct OkxWsOrderEvent {
    arg: OkxWsArg,
    data: Vec<OkxWsOrderData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxWsOrderData {
    inst_id: String,
    ord_id: String,
    cl_ord_id: String,
    px: String,
    sz: String,
    acc_fill_sz: String,
    avg_px: String,
    state: String,
    ord_type: String,
    side: String,
    c_time: String,
    u_time: String,
}

#[derive(Debug, Deserialize)]
struct OkxWsPositionEvent {
    arg: OkxWsArg,
    data: Vec<OkxWsPositionData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxWsPositionData {
    inst_id: String,
    pos_side: String,
    pos: String,
    avg_px: String,
    mark_px: String,
    liq_px: String,
    upl: String,
    lever: String,
}

#[derive(Debug, Deserialize)]
struct OkxWsAccountEvent {
    arg: OkxWsArg,
    data: Vec<OkxWsAccountData>,
}

#[derive(Debug, Deserialize)]
struct OkxWsAccountData {
    details: Vec<OkxWsBalanceDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxWsBalanceDetail {
    ccy: String,
    avail_bal: String,
    frozen_bal: String,
}

// =========================================================================
// Helper Functions
// =========================================================================

fn parse_ws_order_status(status: &str) -> OrderStatus {
    match status {
        "live" => OrderStatus::Open,
        "partially_filled" => OrderStatus::PartiallyFilled,
        "filled" => OrderStatus::Filled,
        "canceled" => OrderStatus::Canceled,
        _ => OrderStatus::Pending,
    }
}

fn parse_ws_order_type(order_type: &str) -> OrderType {
    match order_type {
        "market" => OrderType::Market,
        "limit" => OrderType::Limit,
        "trigger" => OrderType::StopMarket,
        _ => OrderType::Limit,
    }
}
