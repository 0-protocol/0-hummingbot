//! OKX REST API Client
//!
//! Handles all HTTP requests to OKX REST endpoints.

use reqwest::{Client, header::{HeaderMap, HeaderValue}};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, instrument};

use crate::connectors::{
    auth::{hmac_sha256_sign_base64, timestamp_iso, timestamp_ms, ApiCredentials},
    error::ConnectorError,
    types::*,
};

/// OKX REST API client
#[derive(Clone)]
pub struct OkxRestClient {
    client: Client,
    credentials: Option<ApiCredentials>,
    base_url: String,
    is_demo: bool,
}

impl OkxRestClient {
    pub fn new(credentials: ApiCredentials) -> Self {
        Self {
            client: Client::new(),
            credentials: Some(credentials),
            base_url: "https://www.okx.com".to_string(),
            is_demo: false,
        }
    }
    
    pub fn demo(credentials: ApiCredentials) -> Self {
        Self {
            client: Client::new(),
            credentials: Some(credentials),
            base_url: "https://www.okx.com".to_string(),
            is_demo: true,
        }
    }
    
    pub fn public() -> Self {
        Self {
            client: Client::new(),
            credentials: None,
            base_url: "https://www.okx.com".to_string(),
            is_demo: false,
        }
    }
    
    pub fn get_credentials(&self) -> Option<ApiCredentials> {
        self.credentials.clone()
    }
    
    fn build_auth_headers(&self, method: &str, path: &str, body: &str) -> Result<HeaderMap, ConnectorError> {
        let creds = self.credentials.as_ref()
            .ok_or_else(|| ConnectorError::Authentication("No credentials provided".to_string()))?;
        
        let timestamp = timestamp_iso();
        let prehash = format!("{}{}{}{}", timestamp, method, path, body);
        let sign = hmac_sha256_sign_base64(&creds.api_secret, &prehash);
        
        let mut headers = HeaderMap::new();
        headers.insert("OK-ACCESS-KEY", HeaderValue::from_str(&creds.api_key).unwrap());
        headers.insert("OK-ACCESS-SIGN", HeaderValue::from_str(&sign).unwrap());
        headers.insert("OK-ACCESS-TIMESTAMP", HeaderValue::from_str(&timestamp).unwrap());
        
        if let Some(passphrase) = &creds.passphrase {
            headers.insert("OK-ACCESS-PASSPHRASE", HeaderValue::from_str(passphrase).unwrap());
        }
        
        if self.is_demo {
            headers.insert("x-simulated-trading", HeaderValue::from_static("1"));
        }
        
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        Ok(headers)
    }
    
    async fn get_public<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, ConnectorError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("GET {}", url);
        
        let response = self.client.get(&url).send().await?;
        self.handle_response(response).await
    }
    
    async fn get_signed<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, ConnectorError> {
        let headers = self.build_auth_headers("GET", path, "")?;
        let url = format!("{}{}", self.base_url, path);
        
        debug!("GET {} (signed)", url);
        
        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await?;
        
        self.handle_response(response).await
    }
    
    async fn post_signed<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &str) -> Result<T, ConnectorError> {
        let headers = self.build_auth_headers("POST", path, body)?;
        let url = format!("{}{}", self.base_url, path);
        
        debug!("POST {} (signed)", url);
        
        let response = self.client
            .post(&url)
            .headers(headers)
            .body(body.to_string())
            .send()
            .await?;
        
        self.handle_response(response).await
    }
    
    async fn handle_response<T: for<'de> Deserialize<'de>>(&self, response: reqwest::Response) -> Result<T, ConnectorError> {
        let text = response.text().await?;
        
        let wrapper: OkxResponse<T> = serde_json::from_str(&text).map_err(|e| {
            ConnectorError::ParseError(format!("Failed to parse response: {} - Body: {}", e, &text[..text.len().min(200)]))
        })?;
        
        if wrapper.code != "0" {
            match wrapper.code.as_str() {
                "50011" => return Err(ConnectorError::RateLimited { retry_after_ms: None }),
                "50001" => return Err(ConnectorError::Authentication(wrapper.msg)),
                code => return Err(ConnectorError::ExchangeError {
                    code: code.parse().unwrap_or(-1),
                    message: wrapper.msg,
                }),
            }
        }
        
        wrapper.data.ok_or_else(|| ConnectorError::ParseError("Empty data in response".to_string()))
    }
    
    // =========================================================================
    // Market Data Endpoints
    // =========================================================================
    
    #[instrument(skip(self))]
    pub async fn get_ticker(&self, inst_id: &str, pair: &str) -> Result<Ticker, ConnectorError> {
        let path = format!("/api/v5/market/ticker?instId={}", inst_id);
        let resp: Vec<OkxTickerResponse> = self.get_public(&path).await?;
        
        let ticker = resp.into_iter().next()
            .ok_or_else(|| ConnectorError::ParseError("Empty ticker response".to_string()))?;
        
        Ok(Ticker {
            pair: pair.to_string(),
            last_price: Decimal::from_str(&ticker.last).unwrap_or_default(),
            bid: Decimal::from_str(&ticker.bid_px).unwrap_or_default(),
            ask: Decimal::from_str(&ticker.ask_px).unwrap_or_default(),
            high_24h: Decimal::from_str(&ticker.high_24h).unwrap_or_default(),
            low_24h: Decimal::from_str(&ticker.low_24h).unwrap_or_default(),
            volume_24h: Decimal::from_str(&ticker.vol_24h).unwrap_or_default(),
            change_24h: Decimal::ZERO,
            timestamp: ticker.ts.parse().unwrap_or(timestamp_ms() as i64),
        })
    }
    
    #[instrument(skip(self))]
    pub async fn get_orderbook(&self, inst_id: &str, pair: &str, depth: u32) -> Result<OrderBook, ConnectorError> {
        let limit = depth.min(400);
        let path = format!("/api/v5/market/books?instId={}&sz={}", inst_id, limit);
        let resp: Vec<OkxOrderBookResponse> = self.get_public(&path).await?;
        
        let book = resp.into_iter().next()
            .ok_or_else(|| ConnectorError::ParseError("Empty orderbook response".to_string()))?;
        
        let bids = book.bids.iter()
            .map(|level| OrderBookLevel {
                price: Decimal::from_str(&level[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&level[1]).unwrap_or_default(),
            })
            .collect();
        
        let asks = book.asks.iter()
            .map(|level| OrderBookLevel {
                price: Decimal::from_str(&level[0]).unwrap_or_default(),
                quantity: Decimal::from_str(&level[1]).unwrap_or_default(),
            })
            .collect();
        
        Ok(OrderBook {
            pair: pair.to_string(),
            bids,
            asks,
            timestamp: book.ts.parse().unwrap_or(timestamp_ms() as i64),
        })
    }
    
    #[instrument(skip(self))]
    pub async fn get_trades(&self, inst_id: &str, pair: &str, limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        let limit = limit.min(500);
        let path = format!("/api/v5/market/trades?instId={}&limit={}", inst_id, limit);
        let resp: Vec<OkxTradeResponse> = self.get_public(&path).await?;
        
        Ok(resp.into_iter().map(|t| {
            Trade {
                id: t.trade_id,
                pair: pair.to_string(),
                price: Decimal::from_str(&t.px).unwrap_or_default(),
                quantity: Decimal::from_str(&t.sz).unwrap_or_default(),
                side: if t.side == "buy" { OrderSide::Buy } else { OrderSide::Sell },
                timestamp: t.ts.parse().unwrap_or(0),
            }
        }).collect())
    }
    
    // =========================================================================
    // Trading Endpoints
    // =========================================================================
    
    #[instrument(skip(self, order))]
    pub async fn place_order(&self, inst_id: &str, inst_type: &str, order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        let td_mode = if inst_type == "SWAP" { "cross" } else { "cash" };
        
        let side = match order.side {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        };
        
        let ord_type = match order.order_type {
            OrderType::Market => "market",
            OrderType::Limit => "limit",
            OrderType::StopLimit | OrderType::StopMarket => "trigger",
            _ => "limit",
        };
        
        let request = OkxOrderRequest {
            inst_id: inst_id.to_string(),
            td_mode: td_mode.to_string(),
            side: side.to_string(),
            ord_type: ord_type.to_string(),
            sz: order.quantity.to_string(),
            px: order.price.map(|p| p.to_string()),
            cl_ord_id: order.client_order_id.clone(),
            reduce_only: if order.reduce_only { Some(true) } else { None },
        };
        
        let body = serde_json::to_string(&request)
            .map_err(|e| ConnectorError::ParseError(e.to_string()))?;
        
        let resp: Vec<OkxOrderResponse> = self.post_signed("/api/v5/trade/order", &body).await?;
        
        let order_resp = resp.into_iter().next()
            .ok_or_else(|| ConnectorError::ParseError("Empty order response".to_string()))?;
        
        if order_resp.s_code != "0" {
            return Err(ConnectorError::OrderError {
                code: order_resp.s_code.parse().ok(),
                message: order_resp.s_msg,
            });
        }
        
        Ok(OrderResponse {
            order_id: order_resp.ord_id,
            client_order_id: order_resp.cl_ord_id,
            status: OrderStatus::Open,
            filled_quantity: Decimal::ZERO,
            avg_fill_price: None,
            tx_hash: None,
            timestamp: timestamp_ms() as i64,
        })
    }
    
    #[instrument(skip(self))]
    pub async fn cancel_order(&self, inst_id: &str, order_id: &str) -> Result<CancelResponse, ConnectorError> {
        let request = serde_json::json!({
            "instId": inst_id,
            "ordId": order_id,
        });
        
        let body = serde_json::to_string(&request)
            .map_err(|e| ConnectorError::ParseError(e.to_string()))?;
        
        let resp: Vec<OkxCancelResponse> = self.post_signed("/api/v5/trade/cancel-order", &body).await?;
        
        let cancel_resp = resp.into_iter().next()
            .ok_or_else(|| ConnectorError::ParseError("Empty cancel response".to_string()))?;
        
        Ok(CancelResponse {
            order_id: cancel_resp.ord_id,
            success: cancel_resp.s_code == "0",
        })
    }
    
    #[instrument(skip(self))]
    pub async fn get_order(&self, inst_id: &str, pair: &str, order_id: &str) -> Result<Order, ConnectorError> {
        let path = format!("/api/v5/trade/order?instId={}&ordId={}", inst_id, order_id);
        let resp: Vec<OkxOrderDetailsResponse> = self.get_signed(&path).await?;
        
        let o = resp.into_iter().next()
            .ok_or_else(|| ConnectorError::ParseError("Order not found".to_string()))?;
        
        let filled = Decimal::from_str(&o.acc_fill_sz).unwrap_or_default();
        let orig = Decimal::from_str(&o.sz).unwrap_or_default();
        
        Ok(Order {
            order_id: o.ord_id,
            client_order_id: if o.cl_ord_id.is_empty() { None } else { Some(o.cl_ord_id) },
            pair: pair.to_string(),
            side: if o.side == "buy" { OrderSide::Buy } else { OrderSide::Sell },
            order_type: parse_okx_order_type(&o.ord_type),
            quantity: orig,
            price: Decimal::from_str(&o.px).ok(),
            stop_price: None,
            status: parse_okx_order_status(&o.state),
            filled_quantity: filled,
            remaining_quantity: orig - filled,
            avg_fill_price: Decimal::from_str(&o.avg_px).ok(),
            time_in_force: TimeInForce::GTC,
            reduce_only: false,
            created_at: o.c_time.parse().unwrap_or(0),
            updated_at: o.u_time.parse().unwrap_or(0),
        })
    }
    
    #[instrument(skip(self, pair_converter))]
    pub async fn get_open_orders<F>(&self, inst_type: &str, inst_id: Option<&str>, pair_converter: F) -> Result<Vec<Order>, ConnectorError>
    where
        F: Fn(&str) -> String,
    {
        let mut path = format!("/api/v5/trade/orders-pending?instType={}", inst_type);
        if let Some(id) = inst_id {
            path.push_str(&format!("&instId={}", id));
        }
        
        let resp: Vec<OkxOrderDetailsResponse> = self.get_signed(&path).await?;
        
        Ok(resp.into_iter().map(|o| {
            let filled = Decimal::from_str(&o.acc_fill_sz).unwrap_or_default();
            let orig = Decimal::from_str(&o.sz).unwrap_or_default();
            
            Order {
                order_id: o.ord_id,
                client_order_id: if o.cl_ord_id.is_empty() { None } else { Some(o.cl_ord_id) },
                pair: pair_converter(&o.inst_id),
                side: if o.side == "buy" { OrderSide::Buy } else { OrderSide::Sell },
                order_type: parse_okx_order_type(&o.ord_type),
                quantity: orig,
                price: Decimal::from_str(&o.px).ok(),
                stop_price: None,
                status: parse_okx_order_status(&o.state),
                filled_quantity: filled,
                remaining_quantity: orig - filled,
                avg_fill_price: Decimal::from_str(&o.avg_px).ok(),
                time_in_force: TimeInForce::GTC,
                reduce_only: false,
                created_at: o.c_time.parse().unwrap_or(0),
                updated_at: o.u_time.parse().unwrap_or(0),
            }
        }).collect())
    }
    
    // =========================================================================
    // Account Endpoints
    // =========================================================================
    
    #[instrument(skip(self))]
    pub async fn get_balance(&self, asset: &str) -> Result<Balance, ConnectorError> {
        let path = format!("/api/v5/account/balance?ccy={}", asset);
        let resp: Vec<OkxAccountResponse> = self.get_signed(&path).await?;
        
        let account = resp.into_iter().next()
            .ok_or_else(|| ConnectorError::ParseError("Empty balance response".to_string()))?;
        
        account.details.into_iter()
            .find(|b| b.ccy.to_uppercase() == asset.to_uppercase())
            .map(|b| {
                let free = Decimal::from_str(&b.avail_bal).unwrap_or_default();
                let locked = Decimal::from_str(&b.frozen_bal).unwrap_or_default();
                Balance {
                    asset: b.ccy,
                    free,
                    locked,
                    total: free + locked,
                }
            })
            .ok_or_else(|| ConnectorError::InvalidRequest(format!("Asset {} not found", asset)))
    }
    
    #[instrument(skip(self))]
    pub async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        let path = "/api/v5/account/balance";
        let resp: Vec<OkxAccountResponse> = self.get_signed(path).await?;
        
        let account = resp.into_iter().next()
            .ok_or_else(|| ConnectorError::ParseError("Empty balance response".to_string()))?;
        
        Ok(account.details.into_iter()
            .filter_map(|b| {
                let free = Decimal::from_str(&b.avail_bal).ok()?;
                let locked = Decimal::from_str(&b.frozen_bal).ok()?;
                let total = free + locked;
                
                if total.is_zero() {
                    return None;
                }
                
                Some(Balance {
                    asset: b.ccy,
                    free,
                    locked,
                    total,
                })
            })
            .collect())
    }
    
    #[instrument(skip(self, pair_converter))]
    pub async fn get_positions<F>(&self, inst_type: &str, inst_id: Option<&str>, pair_converter: F) -> Result<Vec<Position>, ConnectorError>
    where
        F: Fn(&str) -> String,
    {
        let mut path = format!("/api/v5/account/positions?instType={}", inst_type);
        if let Some(id) = inst_id {
            path.push_str(&format!("&instId={}", id));
        }
        
        let resp: Vec<OkxPositionResponse> = self.get_signed(&path).await?;
        
        Ok(resp.into_iter()
            .filter_map(|p| {
                let qty = Decimal::from_str(&p.pos).ok()?;
                
                if qty.is_zero() {
                    return None;
                }
                
                let side = match p.pos_side.as_str() {
                    "long" => PositionSide::Long,
                    "short" => PositionSide::Short,
                    _ => if qty > Decimal::ZERO { PositionSide::Long } else { PositionSide::Short },
                };
                
                Some(Position {
                    pair: pair_converter(&p.inst_id),
                    side,
                    size: qty.abs(),
                    entry_price: Decimal::from_str(&p.avg_px).unwrap_or_default(),
                    mark_price: Decimal::from_str(&p.mark_px).unwrap_or_default(),
                    liquidation_price: Decimal::from_str(&p.liq_px).ok(),
                    unrealized_pnl: Decimal::from_str(&p.upl).unwrap_or_default(),
                    realized_pnl: Decimal::from_str(&p.realized_pnl).unwrap_or_default(),
                    leverage: Decimal::from_str(&p.lever).unwrap_or(Decimal::ONE),
                    margin: Decimal::from_str(&p.margin.unwrap_or_default()).unwrap_or_default(),
                })
            })
            .collect())
    }
}

// =========================================================================
// OKX Response Types
// =========================================================================

#[derive(Debug, Deserialize)]
struct OkxResponse<T> {
    code: String,
    msg: String,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxTickerResponse {
    last: String,
    bid_px: String,
    ask_px: String,
    vol_24h: String,
    high_24h: String,
    low_24h: String,
    ts: String,
}

#[derive(Debug, Deserialize)]
struct OkxOrderBookResponse {
    asks: Vec<Vec<String>>,
    bids: Vec<Vec<String>>,
    ts: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxTradeResponse {
    trade_id: String,
    px: String,
    sz: String,
    side: String,
    ts: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OkxOrderRequest {
    inst_id: String,
    td_mode: String,
    side: String,
    ord_type: String,
    sz: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    px: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cl_ord_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reduce_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxOrderResponse {
    ord_id: String,
    cl_ord_id: Option<String>,
    s_code: String,
    s_msg: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxCancelResponse {
    ord_id: String,
    s_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxOrderDetailsResponse {
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
struct OkxAccountResponse {
    details: Vec<OkxBalanceDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxBalanceDetail {
    ccy: String,
    avail_bal: String,
    frozen_bal: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxPositionResponse {
    inst_id: String,
    pos_side: String,
    pos: String,
    avg_px: String,
    mark_px: String,
    liq_px: String,
    upl: String,
    realized_pnl: String,
    lever: String,
    margin: Option<String>,
}

// =========================================================================
// Helper Functions
// =========================================================================

fn parse_okx_order_status(status: &str) -> OrderStatus {
    match status {
        "live" => OrderStatus::Open,
        "partially_filled" => OrderStatus::PartiallyFilled,
        "filled" => OrderStatus::Filled,
        "canceled" => OrderStatus::Canceled,
        _ => OrderStatus::Pending,
    }
}

fn parse_okx_order_type(order_type: &str) -> OrderType {
    match order_type {
        "market" => OrderType::Market,
        "limit" => OrderType::Limit,
        "trigger" => OrderType::StopMarket,
        _ => OrderType::Limit,
    }
}
