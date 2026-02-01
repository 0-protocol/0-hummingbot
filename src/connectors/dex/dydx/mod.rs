//! dYdX v4 Connector
//!
//! dYdX v4 is a perpetual DEX built on Cosmos SDK.
//! This connector implements the trading API with Cosmos signing.

mod client;
mod signing;

pub use client::DydxClient;
pub use signing::*;

use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::connectors::dex::DexConnector;
use crate::connectors::{
    Balance, ConnectorBase, ConnectorError, ExchangeType, Order, OrderBook, OrderBookLevel,
    OrderRequest, OrderResponse, OrderSide, OrderStatus, OrderType, Position, Ticker,
    TimeInForce, Trade, TradingPair, TxReceipt, TxStatus,
};
use crate::wallet::CosmosWallet;

/// dYdX v4 API endpoints
pub const DYDX_API_URL: &str = "https://indexer.dydx.trade/v4";
pub const DYDX_TESTNET_API_URL: &str = "https://indexer.v4testnet.dydx.exchange/v4";
pub const DYDX_WS_URL: &str = "wss://indexer.dydx.trade/v4/ws";
pub const DYDX_TESTNET_WS_URL: &str = "wss://indexer.v4testnet.dydx.exchange/v4/ws";

/// dYdX chain ID
pub const DYDX_CHAIN_ID: &str = "dydx-mainnet-1";
pub const DYDX_TESTNET_CHAIN_ID: &str = "dydx-testnet-4";

/// dYdX v4 connector
pub struct DydxConnector {
    /// Cosmos wallet for signing
    wallet: CosmosWallet,
    /// HTTP client
    client: reqwest::Client,
    /// Indexer API URL
    indexer_url: String,
    /// WebSocket URL
    ws_url: String,
    /// Chain ID
    chain_id: String,
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Testnet mode
    testnet: bool,
    /// Subaccount number
    subaccount: u32,
}

impl DydxConnector {
    /// Create a new dYdX v4 connector
    pub fn new(private_key: &str) -> Result<Self, ConnectorError> {
        let wallet = CosmosWallet::for_dydx(private_key)
            .map_err(|e| ConnectorError::WalletError(e.to_string()))?;

        Ok(Self {
            wallet,
            client: reqwest::Client::new(),
            indexer_url: DYDX_API_URL.to_string(),
            ws_url: DYDX_WS_URL.to_string(),
            chain_id: DYDX_CHAIN_ID.to_string(),
            connected: Arc::new(RwLock::new(false)),
            testnet: false,
            subaccount: 0,
        })
    }

    /// Use testnet
    pub fn testnet(mut self) -> Self {
        self.indexer_url = DYDX_TESTNET_API_URL.to_string();
        self.ws_url = DYDX_TESTNET_WS_URL.to_string();
        self.chain_id = DYDX_TESTNET_CHAIN_ID.to_string();
        self.testnet = true;
        self
    }

    /// Set subaccount number
    pub fn with_subaccount(mut self, subaccount: u32) -> Self {
        self.subaccount = subaccount;
        self
    }

    /// Get the dYdX address
    fn address(&self) -> String {
        self.wallet.address().unwrap_or_default()
    }

    /// Get market data from indexer
    async fn get_market(&self, market: &str) -> Result<DydxMarket, ConnectorError> {
        let resp = self
            .client
            .get(&format!("{}/perpetualMarkets/{}", self.indexer_url, market))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ConnectorError::HttpError(
                resp.error_for_status().unwrap_err(),
            ));
        }

        let data: DydxMarketResponse = resp.json().await?;
        Ok(data.market)
    }

    /// Get account info from indexer
    async fn get_account(&self) -> Result<DydxAccount, ConnectorError> {
        let address = self.address();
        let resp = self
            .client
            .get(&format!(
                "{}/addresses/{}/subaccountNumber/{}",
                self.indexer_url, address, self.subaccount
            ))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ConnectorError::HttpError(
                resp.error_for_status().unwrap_err(),
            ));
        }

        let data: DydxAccountResponse = resp.json().await?;
        Ok(data.subaccount)
    }
}

#[async_trait]
impl ConnectorBase for DydxConnector {
    fn name(&self) -> &str {
        "dydx"
    }

    fn exchange_type(&self) -> ExchangeType {
        ExchangeType::Perpetual
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn connect(&mut self) -> Result<(), ConnectorError> {
        // Test connection to indexer
        let resp = self
            .client
            .get(&format!("{}/perpetualMarkets", self.indexer_url))
            .send()
            .await?;

        if resp.status().is_success() {
            *self.connected.write().await = true;
            Ok(())
        } else {
            Err(ConnectorError::HttpError(
                resp.error_for_status().unwrap_err(),
            ))
        }
    }

    async fn disconnect(&mut self) -> Result<(), ConnectorError> {
        *self.connected.write().await = false;
        Ok(())
    }

    async fn get_ticker(&self, pair: &str) -> Result<Ticker, ConnectorError> {
        let market = self.get_market(pair).await?;

        Ok(Ticker {
            pair: pair.to_string(),
            last_price: market.oracle_price.parse().unwrap_or_default(),
            bid: market.oracle_price.parse().unwrap_or_default(), // Would need orderbook for exact bid
            ask: market.oracle_price.parse().unwrap_or_default(), // Would need orderbook for exact ask
            high_24h: Decimal::ZERO,
            low_24h: Decimal::ZERO,
            volume_24h: market.volume_24h.parse().unwrap_or_default(),
            change_24h: market.price_change_24h.parse().unwrap_or_default(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn get_orderbook(&self, pair: &str, depth: u32) -> Result<OrderBook, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/orderbooks/perpetualMarket/{}",
                self.indexer_url, pair
            ))
            .send()
            .await?;

        let book: DydxOrderbookResponse = resp.json().await?;

        let bids: Vec<OrderBookLevel> = book
            .bids
            .iter()
            .take(depth as usize)
            .map(|l| OrderBookLevel {
                price: l.price.parse().unwrap_or_default(),
                quantity: l.size.parse().unwrap_or_default(),
            })
            .collect();

        let asks: Vec<OrderBookLevel> = book
            .asks
            .iter()
            .take(depth as usize)
            .map(|l| OrderBookLevel {
                price: l.price.parse().unwrap_or_default(),
                quantity: l.size.parse().unwrap_or_default(),
            })
            .collect();

        Ok(OrderBook {
            pair: pair.to_string(),
            bids,
            asks,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn get_trades(&self, pair: &str, limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        let resp = self
            .client
            .get(&format!(
                "{}/trades/perpetualMarket/{}?limit={}",
                self.indexer_url, pair, limit
            ))
            .send()
            .await?;

        let data: DydxTradesResponse = resp.json().await?;

        Ok(data
            .trades
            .into_iter()
            .map(|t| Trade {
                id: t.id,
                pair: pair.to_string(),
                price: t.price.parse().unwrap_or_default(),
                quantity: t.size.parse().unwrap_or_default(),
                side: if t.side == "BUY" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                timestamp: t
                    .created_at
                    .parse::<chrono::DateTime<chrono::Utc>>()
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or(0),
            })
            .collect())
    }

    async fn get_pairs(&self) -> Result<Vec<TradingPair>, ConnectorError> {
        let resp = self
            .client
            .get(&format!("{}/perpetualMarkets", self.indexer_url))
            .send()
            .await?;

        let data: DydxMarketsResponse = resp.json().await?;

        Ok(data
            .markets
            .into_iter()
            .map(|(_, market)| TradingPair {
                symbol: market.ticker.clone(),
                base: market
                    .ticker
                    .split('-')
                    .next()
                    .unwrap_or(&market.ticker)
                    .to_string(),
                quote: "USD".to_string(),
                min_order_size: market.step_base_quantums.parse().unwrap_or_default(),
                tick_size: market.subticks_per_tick.parse().unwrap_or_default(),
                step_size: market.step_base_quantums.parse().unwrap_or_default(),
                is_active: market.status == "ACTIVE",
            })
            .collect())
    }

    async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        // dYdX v4 order placement requires Cosmos transaction signing
        // This is a simplified implementation - full implementation needs
        // proper Cosmos TX building and broadcasting

        // For now, return a placeholder indicating the order would be placed
        // Full implementation requires:
        // 1. Build the MsgPlaceOrder message
        // 2. Sign with Cosmos wallet
        // 3. Broadcast to dYdX chain

        Err(ConnectorError::Internal(
            "dYdX v4 order placement requires full Cosmos TX support - use dYdX TypeScript SDK for now".to_string(),
        ))
    }

    async fn cancel_order(&self, order_id: &str, pair: &str) -> Result<(), ConnectorError> {
        // Similar to place_order - requires Cosmos TX
        Err(ConnectorError::Internal(
            "dYdX v4 cancellation requires full Cosmos TX support".to_string(),
        ))
    }

    async fn get_order(&self, order_id: &str, pair: &str) -> Result<Order, ConnectorError> {
        let address = self.address();
        let resp = self
            .client
            .get(&format!(
                "{}/orders/{}",
                self.indexer_url, order_id
            ))
            .send()
            .await?;

        let data: DydxOrderResponse = resp.json().await?;
        let o = data.order;

        Ok(Order {
            order_id: o.id,
            client_order_id: o.client_id,
            pair: o.ticker,
            side: if o.side == "BUY" {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            },
            order_type: match o.order_type.as_str() {
                "LIMIT" => OrderType::Limit,
                "MARKET" => OrderType::Market,
                "STOP_LIMIT" => OrderType::StopLimit,
                "STOP_MARKET" => OrderType::StopMarket,
                _ => OrderType::Limit,
            },
            quantity: o.size.parse().unwrap_or_default(),
            price: o.price.parse().ok(),
            stop_price: o.trigger_price.and_then(|p| p.parse().ok()),
            status: match o.status.as_str() {
                "OPEN" => OrderStatus::Open,
                "FILLED" => OrderStatus::Filled,
                "CANCELED" => OrderStatus::Canceled,
                "PENDING" => OrderStatus::Pending,
                _ => OrderStatus::Pending,
            },
            filled_quantity: o
                .size
                .parse::<Decimal>()
                .unwrap_or_default()
                .saturating_sub(o.remaining_size.parse().unwrap_or_default()),
            remaining_quantity: o.remaining_size.parse().unwrap_or_default(),
            avg_fill_price: None,
            time_in_force: match o.time_in_force.as_str() {
                "GTT" => TimeInForce::GTC,
                "IOC" => TimeInForce::IOC,
                "FOK" => TimeInForce::FOK,
                _ => TimeInForce::GTC,
            },
            reduce_only: o.reduce_only,
            created_at: o
                .created_at
                .parse::<chrono::DateTime<chrono::Utc>>()
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0),
            updated_at: o
                .updated_at
                .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0),
        })
    }

    async fn get_open_orders(&self, pair: Option<&str>) -> Result<Vec<Order>, ConnectorError> {
        let address = self.address();
        let url = match pair {
            Some(p) => format!(
                "{}/orders?address={}&subaccountNumber={}&ticker={}&status=OPEN",
                self.indexer_url, address, self.subaccount, p
            ),
            None => format!(
                "{}/orders?address={}&subaccountNumber={}&status=OPEN",
                self.indexer_url, address, self.subaccount
            ),
        };

        let resp = self.client.get(&url).send().await?;
        let data: DydxOrdersResponse = resp.json().await?;

        Ok(data
            .orders
            .into_iter()
            .map(|o| Order {
                order_id: o.id,
                client_order_id: o.client_id,
                pair: o.ticker,
                side: if o.side == "BUY" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                order_type: OrderType::Limit,
                quantity: o.size.parse().unwrap_or_default(),
                price: o.price.parse().ok(),
                stop_price: None,
                status: OrderStatus::Open,
                filled_quantity: Decimal::ZERO,
                remaining_quantity: o.remaining_size.parse().unwrap_or_default(),
                avg_fill_price: None,
                time_in_force: TimeInForce::GTC,
                reduce_only: o.reduce_only,
                created_at: 0,
                updated_at: 0,
            })
            .collect())
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        let account = self.get_account().await?;

        Ok(vec![Balance {
            asset: "USDC".to_string(),
            free: account.equity.parse().unwrap_or_default(),
            locked: Decimal::ZERO,
            total: account.equity.parse().unwrap_or_default(),
        }])
    }

    async fn get_positions(&self) -> Result<Vec<Position>, ConnectorError> {
        let account = self.get_account().await?;

        Ok(account
            .open_perpetual_positions
            .into_iter()
            .map(|(market, pos)| {
                let size: Decimal = pos.size.parse().unwrap_or_default();
                Position {
                    pair: market,
                    side: if size > Decimal::ZERO {
                        crate::connectors::PositionSide::Long
                    } else {
                        crate::connectors::PositionSide::Short
                    },
                    size: size.abs(),
                    entry_price: pos.entry_price.parse().unwrap_or_default(),
                    mark_price: Decimal::ZERO,
                    liquidation_price: None,
                    unrealized_pnl: pos.unrealized_pnl.parse().unwrap_or_default(),
                    realized_pnl: pos.realized_pnl.parse().unwrap_or_default(),
                    leverage: Decimal::ONE, // dYdX uses cross margin
                    margin: Decimal::ZERO,
                }
            })
            .collect())
    }

    async fn subscribe_orderbook(&self, _pair: &str) -> Result<(), ConnectorError> {
        Ok(())
    }

    async fn subscribe_trades(&self, _pair: &str) -> Result<(), ConnectorError> {
        Ok(())
    }

    async fn subscribe_orders(&self) -> Result<(), ConnectorError> {
        Ok(())
    }
}

#[async_trait]
impl DexConnector for DydxConnector {
    fn wallet_address(&self) -> &str {
        Box::leak(self.address().into_boxed_str())
    }

    fn chain_id(&self) -> u64 {
        // Cosmos chains use string chain IDs, return 0 for dYdX
        0
    }

    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ConnectorError> {
        self.wallet
            .sign(message)
            .map_err(|e| ConnectorError::SigningError(e.to_string()))
    }

    async fn sign_typed_data(&self, _typed_data: &str) -> Result<Vec<u8>, ConnectorError> {
        Err(ConnectorError::Internal(
            "Cosmos does not use EIP-712 typed data".to_string(),
        ))
    }

    async fn estimate_gas(&self, _action: &str) -> Result<Decimal, ConnectorError> {
        // dYdX v4 gas estimation
        Ok(Decimal::new(100000, 0)) // Approximate gas for dYdX operations
    }

    async fn wait_for_confirmation(
        &self,
        tx_hash: &str,
        _confirmations: u32,
    ) -> Result<TxReceipt, ConnectorError> {
        // For Cosmos, we'd query the TX result
        Ok(TxReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: None,
            status: TxStatus::Confirmed,
            gas_used: None,
            gas_price: None,
            fee: None,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
        })
    }

    async fn get_tx_receipt(&self, tx_hash: &str) -> Result<TxReceipt, ConnectorError> {
        Ok(TxReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: None,
            status: TxStatus::Confirmed,
            gas_used: None,
            gas_price: None,
            fee: None,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
        })
    }

    async fn approve_token(
        &self,
        _token_address: &str,
        _spender: &str,
        _amount: Decimal,
    ) -> Result<TxReceipt, ConnectorError> {
        // dYdX v4 uses native USDC - no approval needed
        Ok(TxReceipt {
            tx_hash: String::new(),
            block_number: None,
            status: TxStatus::Confirmed,
            gas_used: None,
            gas_price: None,
            fee: None,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
        })
    }

    async fn check_allowance(
        &self,
        _token_address: &str,
        _spender: &str,
    ) -> Result<Decimal, ConnectorError> {
        Ok(Decimal::MAX)
    }

    async fn deposit(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        Err(ConnectorError::Internal(
            "dYdX v4 deposits require IBC transfer".to_string(),
        ))
    }

    async fn withdraw(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        Err(ConnectorError::Internal(
            "dYdX v4 withdrawals require Cosmos TX".to_string(),
        ))
    }
}

// Response types for dYdX indexer API
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct DydxMarketResponse {
    pub market: DydxMarket,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DydxMarket {
    pub ticker: String,
    pub status: String,
    pub oracle_price: String,
    pub price_change_24h: String,
    pub volume_24h: String,
    pub initial_margin_fraction: String,
    pub maintenance_margin_fraction: String,
    pub step_base_quantums: String,
    pub subticks_per_tick: String,
}

#[derive(Debug, Deserialize)]
pub struct DydxMarketsResponse {
    pub markets: HashMap<String, DydxMarket>,
}

#[derive(Debug, Deserialize)]
pub struct DydxAccountResponse {
    pub subaccount: DydxAccount,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DydxAccount {
    pub address: String,
    pub subaccount_number: u32,
    pub equity: String,
    pub free_collateral: String,
    pub open_perpetual_positions: HashMap<String, DydxPosition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DydxPosition {
    pub market: String,
    pub size: String,
    pub entry_price: String,
    pub unrealized_pnl: String,
    pub realized_pnl: String,
}

#[derive(Debug, Deserialize)]
pub struct DydxOrderbookResponse {
    pub bids: Vec<DydxOrderbookLevel>,
    pub asks: Vec<DydxOrderbookLevel>,
}

#[derive(Debug, Deserialize)]
pub struct DydxOrderbookLevel {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Deserialize)]
pub struct DydxTradesResponse {
    pub trades: Vec<DydxTrade>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DydxTrade {
    pub id: String,
    pub side: String,
    pub size: String,
    pub price: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DydxOrderResponse {
    pub order: DydxOrder,
}

#[derive(Debug, Deserialize)]
pub struct DydxOrdersResponse {
    pub orders: Vec<DydxOrder>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DydxOrder {
    pub id: String,
    pub client_id: Option<String>,
    pub ticker: String,
    pub side: String,
    pub size: String,
    pub price: String,
    pub remaining_size: String,
    pub status: String,
    pub order_type: String,
    pub time_in_force: String,
    pub reduce_only: bool,
    pub trigger_price: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}
