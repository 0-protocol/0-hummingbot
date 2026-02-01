//! Binance REST API Client
//!
//! Handles all HTTP requests to Binance REST endpoints.

use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use tracing::{debug, instrument};

use crate::connectors::{
    auth::{hmac_sha256_sign, timestamp_ms, ApiCredentials},
    error::ConnectorError,
    pair_to_symbol, symbol_to_pair,
    types::*,
};

/// Binance REST API client
#[derive(Clone)]
pub struct BinanceRestClient {
    /// HTTP client
    client: Client,
    /// API credentials (None for public endpoints)
    credentials: Option<ApiCredentials>,
    /// Base URL for REST API
    base_url: String,
    /// Whether this is a futures/perpetual client
    is_perpetual: bool,
}

impl BinanceRestClient {
    /// Create a Spot client with credentials
    pub fn spot(credentials: ApiCredentials) -> Self {
        Self {
            client: Client::new(),
            credentials: Some(credentials),
            base_url: "https://api.binance.com".to_string(),
            is_perpetual: false,
        }
    }
    
    /// Create a Perpetual (Futures) client with credentials
    pub fn perpetual(credentials: ApiCredentials) -> Self {
        Self {
            client: Client::new(),
            credentials: Some(credentials),
            base_url: "https://fapi.binance.com".to_string(),
            is_perpetual: true,
        }
    }
    
    /// Create a Spot testnet client
    pub fn spot_testnet(credentials: ApiCredentials) -> Self {
        Self {
            client: Client::new(),
            credentials: Some(credentials),
            base_url: "https://testnet.binance.vision".to_string(),
            is_perpetual: false,
        }
    }
    
    /// Create a Perpetual testnet client
    pub fn perpetual_testnet(credentials: ApiCredentials) -> Self {
        Self {
            client: Client::new(),
            credentials: Some(credentials),
            base_url: "https://testnet.binancefuture.com".to_string(),
            is_perpetual: true,
        }
    }
    
    /// Create a public Spot client (no auth)
    pub fn spot_public() -> Self {
        Self {
            client: Client::new(),
            credentials: None,
            base_url: "https://api.binance.com".to_string(),
            is_perpetual: false,
        }
    }
    
    /// Create a public Perpetual client (no auth)
    pub fn perpetual_public() -> Self {
        Self {
            client: Client::new(),
            credentials: None,
            base_url: "https://fapi.binance.com".to_string(),
            is_perpetual: true,
        }
    }
    
    /// Sign a request with HMAC-SHA256
    fn sign(&self, query: &str) -> Result<(String, String), ConnectorError> {
        let creds = self.credentials.as_ref()
            .ok_or_else(|| ConnectorError::Authentication("No credentials provided".to_string()))?;
        
        let timestamp = timestamp_ms();
        let query_with_ts = format!("{}&timestamp={}", query, timestamp);
        let signature = hmac_sha256_sign(&creds.api_secret, &query_with_ts);
        
        Ok((query_with_ts, signature))
    }
    
    /// Make a public GET request
    async fn get_public<T: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<T, ConnectorError> {
        let url = format!("{}{}", self.base_url, endpoint);
        debug!("GET {}", url);
        
        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }
    
    /// Make a signed GET request
    async fn get_signed<T: for<'de> Deserialize<'de>>(&self, endpoint: &str, query: &str) -> Result<T, ConnectorError> {
        let creds = self.credentials.as_ref()
            .ok_or_else(|| ConnectorError::Authentication("No credentials provided".to_string()))?;
        
        let (query_with_ts, signature) = self.sign(query)?;
        let url = format!("{}{}", self.base_url, endpoint);
        let full_query = format!("{}&signature={}", query_with_ts, signature);
        
        debug!("GET {} (signed)", url);
        
        let response = self.client
            .get(format!("{}?{}", url, full_query))
            .header("X-MBX-APIKEY", &creds.api_key)
            .send()
            .await?;
        
        self.handle_response(response).await
    }
    
    /// Make a signed POST request
    async fn post_signed<T: for<'de> Deserialize<'de>>(&self, endpoint: &str, query: &str) -> Result<T, ConnectorError> {
        let creds = self.credentials.as_ref()
            .ok_or_else(|| ConnectorError::Authentication("No credentials provided".to_string()))?;
        
        let (query_with_ts, signature) = self.sign(query)?;
        let url = format!("{}{}", self.base_url, endpoint);
        let body = format!("{}&signature={}", query_with_ts, signature);
        
        debug!("POST {} (signed)", url);
        
        let response = self.client
            .post(&url)
            .header("X-MBX-APIKEY", &creds.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;
        
        self.handle_response(response).await
    }
    
    /// Make a signed DELETE request
    async fn delete_signed<T: for<'de> Deserialize<'de>>(&self, endpoint: &str, query: &str) -> Result<T, ConnectorError> {
        let creds = self.credentials.as_ref()
            .ok_or_else(|| ConnectorError::Authentication("No credentials provided".to_string()))?;
        
        let (query_with_ts, signature) = self.sign(query)?;
        let url = format!("{}{}", self.base_url, endpoint);
        
        debug!("DELETE {} (signed)", url);
        
        let response = self.client
            .delete(format!("{}?{}&signature={}", url, query_with_ts, signature))
            .header("X-MBX-APIKEY", &creds.api_key)
            .send()
            .await?;
        
        self.handle_response(response).await
    }
    
    /// Handle API response
    async fn handle_response<T: for<'de> Deserialize<'de>>(&self, response: reqwest::Response) -> Result<T, ConnectorError> {
        let status = response.status();
        let text = response.text().await?;
        
        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<BinanceErrorResponse>(&text) {
                if error.code == -1015 {
                    return Err(ConnectorError::RateLimited { retry_after_ms: None });
                }
                return Err(ConnectorError::ExchangeError {
                    code: error.code,
                    message: error.msg,
                });
            }
            return Err(ConnectorError::Unknown(format!("HTTP {}: {}", status, text)));
        }
        
        serde_json::from_str(&text).map_err(|e| {
            ConnectorError::ParseError(format!("Failed to parse response: {} - Body: {}", e, &text[..text.len().min(200)]))
        })
    }
    
    // =========================================================================
    // Market Data Endpoints
    // =========================================================================
    
    #[instrument(skip(self))]
    pub async fn get_ticker(&self, pair: &str) -> Result<Ticker, ConnectorError> {
        let symbol = pair_to_symbol(pair);
        let endpoint = if self.is_perpetual {
            format!("/fapi/v1/ticker/24hr?symbol={}", symbol)
        } else {
            format!("/api/v3/ticker/24hr?symbol={}", symbol)
        };
        
        let resp: BinanceTickerResponse = self.get_public(&endpoint).await?;
        
        Ok(Ticker {
            pair: pair.to_string(),
            last_price: Decimal::from_str(&resp.last_price).unwrap_or_default(),
            bid: Decimal::from_str(&resp.bid_price).unwrap_or_default(),
            ask: Decimal::from_str(&resp.ask_price).unwrap_or_default(),
            high_24h: Decimal::from_str(&resp.high_price).unwrap_or_default(),
            low_24h: Decimal::from_str(&resp.low_price).unwrap_or_default(),
            volume_24h: Decimal::from_str(&resp.volume).unwrap_or_default(),
            change_24h: Decimal::from_str(&resp.price_change_percent).unwrap_or_default(),
            timestamp: timestamp_ms() as i64,
        })
    }
    
    #[instrument(skip(self))]
    pub async fn get_orderbook(&self, pair: &str, depth: u32) -> Result<OrderBook, ConnectorError> {
        let symbol = pair_to_symbol(pair);
        let limit = depth.min(1000);
        
        let endpoint = if self.is_perpetual {
            format!("/fapi/v1/depth?symbol={}&limit={}", symbol, limit)
        } else {
            format!("/api/v3/depth?symbol={}&limit={}", symbol, limit)
        };
        
        let resp: BinanceOrderBookResponse = self.get_public(&endpoint).await?;
        
        let bids = resp.bids.iter()
            .map(|[price, qty]| OrderBookLevel {
                price: Decimal::from_str(price).unwrap_or_default(),
                quantity: Decimal::from_str(qty).unwrap_or_default(),
            })
            .collect();
        
        let asks = resp.asks.iter()
            .map(|[price, qty]| OrderBookLevel {
                price: Decimal::from_str(price).unwrap_or_default(),
                quantity: Decimal::from_str(qty).unwrap_or_default(),
            })
            .collect();
        
        Ok(OrderBook {
            pair: pair.to_string(),
            bids,
            asks,
            timestamp: timestamp_ms() as i64,
        })
    }
    
    #[instrument(skip(self))]
    pub async fn get_trades(&self, pair: &str, limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        let symbol = pair_to_symbol(pair);
        let limit = limit.min(1000);
        
        let endpoint = if self.is_perpetual {
            format!("/fapi/v1/trades?symbol={}&limit={}", symbol, limit)
        } else {
            format!("/api/v3/trades?symbol={}&limit={}", symbol, limit)
        };
        
        let resp: Vec<BinanceTradeResponse> = self.get_public(&endpoint).await?;
        
        Ok(resp.into_iter().map(|t| {
            Trade {
                id: t.id.to_string(),
                pair: pair.to_string(),
                price: Decimal::from_str(&t.price).unwrap_or_default(),
                quantity: Decimal::from_str(&t.qty).unwrap_or_default(),
                side: if t.is_buyer_maker { OrderSide::Sell } else { OrderSide::Buy },
                timestamp: t.time,
            }
        }).collect())
    }
    
    // =========================================================================
    // Trading Endpoints
    // =========================================================================
    
    #[instrument(skip(self, order))]
    pub async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        let symbol = pair_to_symbol(&order.pair);
        
        let side = match order.side {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        };
        
        let order_type = match order.order_type {
            OrderType::Market => "MARKET",
            OrderType::Limit => "LIMIT",
            OrderType::StopLimit => "STOP_LOSS_LIMIT",
            OrderType::StopMarket => "STOP_MARKET",
            _ => "LIMIT",
        };
        
        let mut params = vec![
            format!("symbol={}", symbol),
            format!("side={}", side),
            format!("type={}", order_type),
            format!("quantity={}", order.quantity),
        ];
        
        if let Some(price) = &order.price {
            params.push(format!("price={}", price));
        }
        
        if let Some(client_id) = &order.client_order_id {
            params.push(format!("newClientOrderId={}", client_id));
        }
        
        if matches!(order.order_type, OrderType::Limit) {
            let tif = match order.time_in_force {
                TimeInForce::GTC => "GTC",
                TimeInForce::IOC => "IOC",
                TimeInForce::FOK => "FOK",
                _ => "GTC",
            };
            params.push(format!("timeInForce={}", tif));
        }
        
        if self.is_perpetual && order.reduce_only {
            params.push("reduceOnly=true".to_string());
        }
        
        let query = params.join("&");
        let endpoint = if self.is_perpetual {
            "/fapi/v1/order"
        } else {
            "/api/v3/order"
        };
        
        let resp: BinanceOrderResponse = self.post_signed(endpoint, &query).await?;
        
        Ok(OrderResponse {
            order_id: resp.order_id.to_string(),
            client_order_id: resp.client_order_id,
            status: parse_order_status(&resp.status),
            filled_quantity: Decimal::from_str(&resp.executed_qty.unwrap_or_default()).unwrap_or_default(),
            avg_fill_price: resp.avg_price.and_then(|p| Decimal::from_str(&p).ok()),
            tx_hash: None,
            timestamp: resp.transact_time.unwrap_or(timestamp_ms() as i64),
        })
    }
    
    #[instrument(skip(self))]
    pub async fn cancel_order(&self, pair: &str, order_id: &str) -> Result<CancelResponse, ConnectorError> {
        let symbol = pair_to_symbol(pair);
        let query = format!("symbol={}&orderId={}", symbol, order_id);
        
        let endpoint = if self.is_perpetual {
            "/fapi/v1/order"
        } else {
            "/api/v3/order"
        };
        
        let resp: BinanceCancelResponse = self.delete_signed(endpoint, &query).await?;
        
        Ok(CancelResponse {
            order_id: resp.order_id.to_string(),
            success: resp.status == "CANCELED",
        })
    }
    
    #[instrument(skip(self))]
    pub async fn get_order(&self, pair: &str, order_id: &str) -> Result<Order, ConnectorError> {
        let symbol = pair_to_symbol(pair);
        let query = format!("symbol={}&orderId={}", symbol, order_id);
        
        let endpoint = if self.is_perpetual {
            "/fapi/v1/order"
        } else {
            "/api/v3/order"
        };
        
        let resp: BinanceOrderDetailsResponse = self.get_signed(endpoint, &query).await?;
        let filled = Decimal::from_str(&resp.executed_qty).unwrap_or_default();
        let orig = Decimal::from_str(&resp.orig_qty).unwrap_or_default();
        
        Ok(Order {
            order_id: resp.order_id.to_string(),
            client_order_id: resp.client_order_id,
            pair: pair.to_string(),
            side: if resp.side == "BUY" { OrderSide::Buy } else { OrderSide::Sell },
            order_type: parse_order_type(&resp.order_type),
            quantity: orig,
            price: Decimal::from_str(&resp.price).ok(),
            stop_price: None,
            status: parse_order_status(&resp.status),
            filled_quantity: filled,
            remaining_quantity: orig - filled,
            avg_fill_price: resp.avg_price.and_then(|p| Decimal::from_str(&p).ok()),
            time_in_force: TimeInForce::GTC,
            reduce_only: false,
            created_at: resp.time,
            updated_at: resp.update_time,
        })
    }
    
    #[instrument(skip(self))]
    pub async fn get_open_orders(&self, pair: Option<&str>) -> Result<Vec<Order>, ConnectorError> {
        let query = if let Some(p) = pair {
            format!("symbol={}", pair_to_symbol(p))
        } else {
            String::new()
        };
        
        let endpoint = if self.is_perpetual {
            "/fapi/v1/openOrders"
        } else {
            "/api/v3/openOrders"
        };
        
        let resp: Vec<BinanceOrderDetailsResponse> = self.get_signed(endpoint, &query).await?;
        
        Ok(resp.into_iter().map(|o| {
            let p = pair.map(|s| s.to_string()).unwrap_or_else(|| symbol_to_pair(&o.symbol, 4));
            let filled = Decimal::from_str(&o.executed_qty).unwrap_or_default();
            let orig = Decimal::from_str(&o.orig_qty).unwrap_or_default();
            
            Order {
                order_id: o.order_id.to_string(),
                client_order_id: o.client_order_id,
                pair: p,
                side: if o.side == "BUY" { OrderSide::Buy } else { OrderSide::Sell },
                order_type: parse_order_type(&o.order_type),
                quantity: orig,
                price: Decimal::from_str(&o.price).ok(),
                stop_price: None,
                status: parse_order_status(&o.status),
                filled_quantity: filled,
                remaining_quantity: orig - filled,
                avg_fill_price: o.avg_price.and_then(|p| Decimal::from_str(&p).ok()),
                time_in_force: TimeInForce::GTC,
                reduce_only: false,
                created_at: o.time,
                updated_at: o.update_time,
            }
        }).collect())
    }
    
    #[instrument(skip(self))]
    pub async fn cancel_all_orders(&self, pair: Option<&str>) -> Result<u32, ConnectorError> {
        if self.is_perpetual {
            if let Some(p) = pair {
                let symbol = pair_to_symbol(p);
                let query = format!("symbol={}", symbol);
                let _: serde_json::Value = self.delete_signed("/fapi/v1/allOpenOrders", &query).await?;
                Ok(0)
            } else {
                Err(ConnectorError::InvalidRequest("Symbol required for cancel all on futures".to_string()))
            }
        } else {
            let orders = self.get_open_orders(pair).await?;
            let count = orders.len();
            for order in orders {
                let _ = self.cancel_order(&order.pair, &order.order_id).await;
            }
            Ok(count as u32)
        }
    }
    
    // =========================================================================
    // Account Endpoints
    // =========================================================================
    
    #[instrument(skip(self))]
    pub async fn get_balance(&self, asset: &str) -> Result<Balance, ConnectorError> {
        let balances = self.get_balances().await?;
        balances.into_iter()
            .find(|b| b.asset.to_uppercase() == asset.to_uppercase())
            .ok_or_else(|| ConnectorError::InvalidRequest(format!("Asset {} not found", asset)))
    }
    
    #[instrument(skip(self))]
    pub async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        let query = "";
        
        if self.is_perpetual {
            let resp: Vec<BinanceFuturesBalanceResponse> = self.get_signed("/fapi/v2/balance", query).await?;
            
            Ok(resp.into_iter()
                .filter_map(|b| {
                    let free = Decimal::from_str(&b.available_balance).ok()?;
                    let total = Decimal::from_str(&b.balance).ok()?;
                    let locked = total - free;
                    
                    if total.is_zero() {
                        return None;
                    }
                    
                    Some(Balance {
                        asset: b.asset,
                        free,
                        locked,
                        total,
                    })
                })
                .collect())
        } else {
            let resp: BinanceAccountResponse = self.get_signed("/api/v3/account", query).await?;
            
            Ok(resp.balances.into_iter()
                .filter_map(|b| {
                    let free = Decimal::from_str(&b.free).ok()?;
                    let locked = Decimal::from_str(&b.locked).ok()?;
                    let total = free + locked;
                    
                    if total.is_zero() {
                        return None;
                    }
                    
                    Some(Balance {
                        asset: b.asset,
                        free,
                        locked,
                        total,
                    })
                })
                .collect())
        }
    }
    
    #[instrument(skip(self))]
    pub async fn get_positions(&self, pair: Option<&str>) -> Result<Vec<Position>, ConnectorError> {
        if !self.is_perpetual {
            return Ok(vec![]);
        }
        
        let query = "";
        let resp: Vec<BinancePositionResponse> = self.get_signed("/fapi/v2/positionRisk", query).await?;
        
        Ok(resp.into_iter()
            .filter_map(|p| {
                let qty = Decimal::from_str(&p.position_amt).ok()?;
                
                if qty.is_zero() {
                    return None;
                }
                
                if let Some(filter_pair) = pair {
                    if pair_to_symbol(filter_pair) != p.symbol {
                        return None;
                    }
                }
                
                let side = if qty > Decimal::ZERO {
                    PositionSide::Long
                } else {
                    PositionSide::Short
                };
                
                Some(Position {
                    pair: symbol_to_pair(&p.symbol, 4),
                    side,
                    size: qty.abs(),
                    entry_price: Decimal::from_str(&p.entry_price).unwrap_or_default(),
                    mark_price: Decimal::from_str(&p.mark_price).unwrap_or_default(),
                    liquidation_price: Decimal::from_str(&p.liquidation_price).ok(),
                    unrealized_pnl: Decimal::from_str(&p.unrealized_profit).unwrap_or_default(),
                    realized_pnl: Decimal::ZERO,
                    leverage: Decimal::from_str(&p.leverage).unwrap_or(Decimal::ONE),
                    margin: Decimal::from_str(&p.isolated_margin.unwrap_or_default()).unwrap_or_default(),
                })
            })
            .collect())
    }
    
    /// Get user data stream listen key
    pub async fn get_listen_key(&self) -> Result<String, ConnectorError> {
        let creds = self.credentials.as_ref()
            .ok_or_else(|| ConnectorError::Authentication("No credentials provided".to_string()))?;
        
        let endpoint = if self.is_perpetual {
            "/fapi/v1/listenKey"
        } else {
            "/api/v3/userDataStream"
        };
        
        let url = format!("{}{}", self.base_url, endpoint);
        
        let response = self.client
            .post(&url)
            .header("X-MBX-APIKEY", &creds.api_key)
            .send()
            .await?;
        
        let resp: BinanceListenKeyResponse = self.handle_response(response).await?;
        Ok(resp.listen_key)
    }
}

// =========================================================================
// Binance Response Types
// =========================================================================

#[derive(Debug, Deserialize)]
struct BinanceErrorResponse {
    code: i32,
    msg: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceTickerResponse {
    bid_price: String,
    ask_price: String,
    last_price: String,
    volume: String,
    high_price: String,
    low_price: String,
    price_change_percent: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceOrderBookResponse {
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceTradeResponse {
    id: u64,
    price: String,
    qty: String,
    time: i64,
    is_buyer_maker: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceOrderResponse {
    order_id: u64,
    client_order_id: Option<String>,
    status: String,
    executed_qty: Option<String>,
    avg_price: Option<String>,
    transact_time: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceCancelResponse {
    order_id: u64,
    status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceOrderDetailsResponse {
    symbol: String,
    order_id: u64,
    client_order_id: Option<String>,
    price: String,
    orig_qty: String,
    executed_qty: String,
    avg_price: Option<String>,
    status: String,
    #[serde(rename = "type")]
    order_type: String,
    side: String,
    time: i64,
    update_time: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceAccountResponse {
    balances: Vec<BinanceBalanceResponse>,
}

#[derive(Debug, Deserialize)]
struct BinanceBalanceResponse {
    asset: String,
    free: String,
    locked: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceFuturesBalanceResponse {
    asset: String,
    balance: String,
    available_balance: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinancePositionResponse {
    symbol: String,
    position_amt: String,
    entry_price: String,
    mark_price: String,
    unrealized_profit: String,
    liquidation_price: String,
    leverage: String,
    isolated_margin: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceListenKeyResponse {
    listen_key: String,
}

// =========================================================================
// Helper Functions
// =========================================================================

fn parse_order_status(status: &str) -> OrderStatus {
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

fn parse_order_type(order_type: &str) -> OrderType {
    match order_type {
        "MARKET" => OrderType::Market,
        "LIMIT" => OrderType::Limit,
        "STOP_LOSS_LIMIT" | "STOP_LIMIT" => OrderType::StopLimit,
        "STOP_MARKET" => OrderType::StopMarket,
        "TAKE_PROFIT" | "TAKE_PROFIT_LIMIT" => OrderType::TakeProfit,
        _ => OrderType::Limit,
    }
}
