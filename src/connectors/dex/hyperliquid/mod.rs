//! Hyperliquid Connector
//!
//! Hyperliquid is a high-performance perpetual DEX on Arbitrum.
//! This connector implements the full trading API with EIP-712 signing.

mod api;
mod signing;
mod types;

pub use api::HyperliquidApi;
pub use signing::{build_order_action, sign_l1_action};
pub use types::*;

use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::connectors::dex::DexConnector;
use crate::connectors::{
    Balance, ConnectorBase, ConnectorError, ExchangeType, Order, OrderBook, OrderBookLevel,
    OrderRequest, OrderResponse, OrderSide, OrderStatus, Position, Ticker, Trade, TradingPair,
    TxReceipt, TxStatus,
};
use crate::wallet::EvmWallet;

/// Hyperliquid chain ID (Arbitrum One)
pub const HYPERLIQUID_CHAIN_ID: u64 = 42161;

/// Hyperliquid API endpoints
pub const HYPERLIQUID_API_URL: &str = "https://api.hyperliquid.xyz";
pub const HYPERLIQUID_TESTNET_API_URL: &str = "https://api.hyperliquid-testnet.xyz";
pub const HYPERLIQUID_WS_URL: &str = "wss://api.hyperliquid.xyz/ws";
pub const HYPERLIQUID_TESTNET_WS_URL: &str = "wss://api.hyperliquid-testnet.xyz/ws";

/// Hyperliquid connector for perpetual trading
pub struct HyperliquidConnector {
    /// EVM wallet for signing
    wallet: EvmWallet,
    /// HTTP client
    client: reqwest::Client,
    /// API base URL
    api_url: String,
    /// WebSocket URL
    ws_url: String,
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Testnet mode
    testnet: bool,
    /// Cached asset metadata
    asset_meta: Arc<RwLock<HashMap<String, AssetMeta>>>,
}

impl HyperliquidConnector {
    /// Create a new Hyperliquid connector
    pub fn new(private_key: &str) -> Result<Self, ConnectorError> {
        let wallet = EvmWallet::from_private_key(private_key, HYPERLIQUID_CHAIN_ID)
            .map_err(|e| ConnectorError::WalletError(e.to_string()))?;

        Ok(Self {
            wallet,
            client: reqwest::Client::new(),
            api_url: HYPERLIQUID_API_URL.to_string(),
            ws_url: HYPERLIQUID_WS_URL.to_string(),
            connected: Arc::new(RwLock::new(false)),
            testnet: false,
            asset_meta: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Use testnet
    pub fn testnet(mut self) -> Self {
        self.api_url = HYPERLIQUID_TESTNET_API_URL.to_string();
        self.ws_url = HYPERLIQUID_TESTNET_WS_URL.to_string();
        self.testnet = true;
        self
    }

    /// Get asset metadata (index, decimals, etc.)
    async fn get_asset_meta(&self, symbol: &str) -> Result<AssetMeta, ConnectorError> {
        // Check cache first
        {
            let cache = self.asset_meta.read().await;
            if let Some(meta) = cache.get(symbol) {
                return Ok(meta.clone());
            }
        }

        // Fetch from API
        let body = serde_json::json!({
            "type": "meta"
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let meta_response: MetaResponse = resp.json().await?;

        // Cache all assets
        {
            let mut cache = self.asset_meta.write().await;
            for (idx, asset) in meta_response.universe.iter().enumerate() {
                let meta = AssetMeta {
                    index: idx as u32,
                    name: asset.name.clone(),
                    sz_decimals: asset.sz_decimals,
                    max_leverage: asset.max_leverage,
                };
                cache.insert(asset.name.clone(), meta);
            }
        }

        let cache = self.asset_meta.read().await;
        cache
            .get(symbol)
            .cloned()
            .ok_or_else(|| ConnectorError::NotFound(format!("Asset {} not found", symbol)))
    }

    /// Get all mid prices
    async fn get_all_mids(&self) -> Result<HashMap<String, Decimal>, ConnectorError> {
        let body = serde_json::json!({
            "type": "allMids"
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let mids: HashMap<String, String> = resp.json().await?;

        mids.into_iter()
            .map(|(k, v)| {
                let price = v
                    .parse::<Decimal>()
                    .map_err(|_| ConnectorError::InvalidResponse("Invalid price".to_string()))?;
                Ok((k, price))
            })
            .collect()
    }

    /// Get user state (balances and positions)
    async fn get_user_state(&self) -> Result<UserState, ConnectorError> {
        let body = serde_json::json!({
            "type": "clearinghouseState",
            "user": self.wallet.address_string()
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let state: UserState = resp.json().await?;
        Ok(state)
    }

    /// Execute an order action
    async fn execute_action(&self, action: serde_json::Value) -> Result<String, ConnectorError> {
        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let signature = sign_l1_action(&self.wallet, &action, nonce, !self.testnet).await?;

        let body = serde_json::json!({
            "action": action,
            "nonce": nonce,
            "signature": signature,
            "vaultAddress": null
        });

        let resp = self
            .client
            .post(&format!("{}/exchange", self.api_url))
            .json(&body)
            .send()
            .await?;

        let result: ExchangeResponse = resp.json().await?;

        match result.status.as_str() {
            "ok" => Ok(result
                .response
                .and_then(|r| r.data.and_then(|d| d.statuses.first().cloned()))
                .unwrap_or_default()),
            _ => Err(ConnectorError::OrderRejected(
                result.response.map_or("Unknown error".to_string(), |r| {
                    format!("{:?}", r)
                }),
            )),
        }
    }
}

#[async_trait]
impl ConnectorBase for HyperliquidConnector {
    fn name(&self) -> &str {
        "hyperliquid"
    }

    fn exchange_type(&self) -> ExchangeType {
        ExchangeType::Perpetual
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn connect(&mut self) -> Result<(), ConnectorError> {
        // Verify we can reach the API
        let body = serde_json::json!({
            "type": "meta"
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
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
        let mids = self.get_all_mids().await?;

        let mid_price = mids
            .get(pair)
            .cloned()
            .ok_or_else(|| ConnectorError::NotFound(format!("Pair {} not found", pair)))?;

        // Get L2 book for bid/ask
        let body = serde_json::json!({
            "type": "l2Book",
            "coin": pair
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let book: L2BookResponse = resp.json().await?;

        let bid = book
            .levels
            .first()
            .and_then(|b| b.first())
            .map(|l| l.px.parse::<Decimal>().unwrap_or_default())
            .unwrap_or_default();

        let ask = book
            .levels
            .get(1)
            .and_then(|a| a.first())
            .map(|l| l.px.parse::<Decimal>().unwrap_or_default())
            .unwrap_or_default();

        Ok(Ticker {
            pair: pair.to_string(),
            last_price: mid_price,
            bid,
            ask,
            high_24h: Decimal::ZERO,   // Not provided by basic API
            low_24h: Decimal::ZERO,    // Not provided by basic API
            volume_24h: Decimal::ZERO, // Not provided by basic API
            change_24h: Decimal::ZERO, // Not provided by basic API
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn get_orderbook(&self, pair: &str, depth: u32) -> Result<OrderBook, ConnectorError> {
        let body = serde_json::json!({
            "type": "l2Book",
            "coin": pair
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let book: L2BookResponse = resp.json().await?;

        let bids: Vec<OrderBookLevel> = book
            .levels
            .first()
            .map(|levels| {
                levels
                    .iter()
                    .take(depth as usize)
                    .map(|l| OrderBookLevel {
                        price: l.px.parse().unwrap_or_default(),
                        quantity: l.sz.parse().unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let asks: Vec<OrderBookLevel> = book
            .levels
            .get(1)
            .map(|levels| {
                levels
                    .iter()
                    .take(depth as usize)
                    .map(|l| OrderBookLevel {
                        price: l.px.parse().unwrap_or_default(),
                        quantity: l.sz.parse().unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(OrderBook {
            pair: pair.to_string(),
            bids,
            asks,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn get_trades(&self, pair: &str, limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        let body = serde_json::json!({
            "type": "recentTrades",
            "coin": pair
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let trades: Vec<HlTrade> = resp.json().await?;

        Ok(trades
            .into_iter()
            .take(limit as usize)
            .map(|t| Trade {
                id: t.tid.to_string(),
                pair: pair.to_string(),
                price: t.px.parse().unwrap_or_default(),
                quantity: t.sz.parse().unwrap_or_default(),
                side: if t.side == "B" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                timestamp: t.time,
            })
            .collect())
    }

    async fn get_pairs(&self) -> Result<Vec<TradingPair>, ConnectorError> {
        let body = serde_json::json!({
            "type": "meta"
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let meta: MetaResponse = resp.json().await?;

        Ok(meta
            .universe
            .into_iter()
            .map(|asset| TradingPair {
                symbol: asset.name.clone(),
                base: asset.name,
                quote: "USD".to_string(),
                min_order_size: Decimal::new(1, asset.sz_decimals),
                tick_size: Decimal::new(1, 5), // Hyperliquid uses 5 decimal places for price
                step_size: Decimal::new(1, asset.sz_decimals),
                is_active: true,
            })
            .collect())
    }

    async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        let asset_meta = self.get_asset_meta(&order.pair).await?;

        let is_buy = matches!(order.side, OrderSide::Buy);
        let limit_px = order
            .price
            .map(|p| p.to_string())
            .unwrap_or_else(|| "0".to_string());

        let order_type = if order.price.is_some() {
            serde_json::json!({
                "limit": {
                    "tif": match order.time_in_force {
                        crate::connectors::TimeInForce::GTC => "Gtc",
                        crate::connectors::TimeInForce::IOC => "Ioc",
                        crate::connectors::TimeInForce::FOK => "Fok",
                        crate::connectors::TimeInForce::PostOnly => "Alo",
                        _ => "Gtc"
                    }
                }
            })
        } else {
            serde_json::json!({
                "limit": {
                    "tif": "Ioc"  // Market orders are IOC with aggressive price
                }
            })
        };

        let action = build_order_action(
            asset_meta.index,
            is_buy,
            &limit_px,
            &order.quantity.to_string(),
            order_type,
            order.reduce_only,
        );

        let result = self.execute_action(action).await?;

        Ok(OrderResponse {
            order_id: result,
            client_order_id: order.client_order_id.clone(),
            status: OrderStatus::Open,
            filled_quantity: Decimal::ZERO,
            avg_fill_price: None,
            tx_hash: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn cancel_order(&self, order_id: &str, pair: &str) -> Result<(), ConnectorError> {
        let asset_meta = self.get_asset_meta(pair).await?;

        let oid: u64 = order_id
            .parse()
            .map_err(|_| ConnectorError::InvalidResponse("Invalid order ID".to_string()))?;

        let action = serde_json::json!({
            "type": "cancel",
            "cancels": [{
                "a": asset_meta.index,
                "o": oid
            }]
        });

        self.execute_action(action).await?;
        Ok(())
    }

    async fn get_order(&self, order_id: &str, _pair: &str) -> Result<Order, ConnectorError> {
        let body = serde_json::json!({
            "type": "orderStatus",
            "user": self.wallet.address_string(),
            "oid": order_id.parse::<u64>().unwrap_or(0)
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let status: HlOrderStatus = resp.json().await?;

        Ok(Order {
            order_id: order_id.to_string(),
            client_order_id: status.order.cloid,
            pair: status.order.coin,
            side: if status.order.side == "B" {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            },
            order_type: crate::connectors::OrderType::Limit,
            quantity: status.order.sz.parse().unwrap_or_default(),
            price: Some(status.order.limit_px.parse().unwrap_or_default()),
            stop_price: None,
            status: match status.status.as_str() {
                "open" => OrderStatus::Open,
                "filled" => OrderStatus::Filled,
                "canceled" => OrderStatus::Canceled,
                _ => OrderStatus::Pending,
            },
            filled_quantity: status
                .order
                .sz
                .parse::<Decimal>()
                .unwrap_or_default()
                .saturating_sub(
                    status
                        .order
                        .sz
                        .parse::<Decimal>()
                        .unwrap_or_default(),
                ),
            remaining_quantity: status.order.sz.parse().unwrap_or_default(),
            avg_fill_price: None,
            time_in_force: crate::connectors::TimeInForce::GTC,
            reduce_only: false,
            created_at: status.order.timestamp,
            updated_at: status.order.timestamp,
        })
    }

    async fn get_open_orders(&self, pair: Option<&str>) -> Result<Vec<Order>, ConnectorError> {
        let body = serde_json::json!({
            "type": "openOrders",
            "user": self.wallet.address_string()
        });

        let resp = self
            .client
            .post(&format!("{}/info", self.api_url))
            .json(&body)
            .send()
            .await?;

        let orders: Vec<HlOpenOrder> = resp.json().await?;

        Ok(orders
            .into_iter()
            .filter(|o| pair.map_or(true, |p| o.coin == p))
            .map(|o| Order {
                order_id: o.oid.to_string(),
                client_order_id: o.cloid,
                pair: o.coin,
                side: if o.side == "B" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                order_type: crate::connectors::OrderType::Limit,
                quantity: o.sz.parse().unwrap_or_default(),
                price: Some(o.limit_px.parse().unwrap_or_default()),
                stop_price: None,
                status: OrderStatus::Open,
                filled_quantity: Decimal::ZERO,
                remaining_quantity: o.sz.parse().unwrap_or_default(),
                avg_fill_price: None,
                time_in_force: crate::connectors::TimeInForce::GTC,
                reduce_only: false,
                created_at: o.timestamp,
                updated_at: o.timestamp,
            })
            .collect())
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        let state = self.get_user_state().await?;

        Ok(vec![Balance {
            asset: "USDC".to_string(),
            free: state
                .margin_summary
                .account_value
                .parse()
                .unwrap_or_default(),
            locked: Decimal::ZERO,
            total: state
                .margin_summary
                .account_value
                .parse()
                .unwrap_or_default(),
        }])
    }

    async fn get_positions(&self) -> Result<Vec<Position>, ConnectorError> {
        let state = self.get_user_state().await?;

        Ok(state
            .asset_positions
            .into_iter()
            .filter(|p| {
                p.position
                    .szi
                    .parse::<Decimal>()
                    .map(|s| s != Decimal::ZERO)
                    .unwrap_or(false)
            })
            .map(|p| {
                let size: Decimal = p.position.szi.parse().unwrap_or_default();
                Position {
                    pair: p.position.coin,
                    side: if size > Decimal::ZERO {
                        crate::connectors::PositionSide::Long
                    } else {
                        crate::connectors::PositionSide::Short
                    },
                    size: size.abs(),
                    entry_price: p.position.entry_px.unwrap_or_default().parse().unwrap_or_default(),
                    mark_price: Decimal::ZERO, // Would need separate call
                    liquidation_price: p.position.liquidation_px.and_then(|px| px.parse().ok()),
                    unrealized_pnl: p.position.unrealized_pnl.parse().unwrap_or_default(),
                    realized_pnl: Decimal::ZERO,
                    leverage: p.position.leverage.value.parse().unwrap_or_default(),
                    margin: p.position.margin_used.parse().unwrap_or_default(),
                }
            })
            .collect())
    }

    async fn subscribe_orderbook(&self, _pair: &str) -> Result<(), ConnectorError> {
        // WebSocket subscription - to be implemented
        Ok(())
    }

    async fn subscribe_trades(&self, _pair: &str) -> Result<(), ConnectorError> {
        // WebSocket subscription - to be implemented
        Ok(())
    }

    async fn subscribe_orders(&self) -> Result<(), ConnectorError> {
        // WebSocket subscription - to be implemented
        Ok(())
    }
}

#[async_trait]
impl DexConnector for HyperliquidConnector {
    fn wallet_address(&self) -> &str {
        // Return a static string by leaking memory (acceptable for address)
        Box::leak(self.wallet.address_string().into_boxed_str())
    }

    fn chain_id(&self) -> u64 {
        HYPERLIQUID_CHAIN_ID
    }

    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ConnectorError> {
        let signature = self
            .wallet
            .sign_message(message)
            .await
            .map_err(|e| ConnectorError::SigningError(e.to_string()))?;

        Ok(signature.to_vec())
    }

    async fn sign_typed_data(&self, _typed_data: &str) -> Result<Vec<u8>, ConnectorError> {
        // EIP-712 signing is handled internally by sign_l1_action
        Err(ConnectorError::Internal(
            "Use sign_l1_action for Hyperliquid typed data".to_string(),
        ))
    }

    async fn estimate_gas(&self, _action: &str) -> Result<Decimal, ConnectorError> {
        // Hyperliquid doesn't require gas estimation - it's a rollup
        Ok(Decimal::ZERO)
    }

    async fn wait_for_confirmation(
        &self,
        _tx_hash: &str,
        _confirmations: u32,
    ) -> Result<TxReceipt, ConnectorError> {
        // Hyperliquid orders are confirmed instantly
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

    async fn get_tx_receipt(&self, _tx_hash: &str) -> Result<TxReceipt, ConnectorError> {
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

    async fn approve_token(
        &self,
        _token_address: &str,
        _spender: &str,
        _amount: Decimal,
    ) -> Result<TxReceipt, ConnectorError> {
        // Hyperliquid uses USDC bridged to L2 - no approval needed
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
        // No allowance needed for Hyperliquid
        Ok(Decimal::MAX)
    }

    async fn deposit(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        // Deposit requires bridging USDC to Hyperliquid L2
        // This would need on-chain interaction with the bridge contract
        Err(ConnectorError::Internal(
            "Deposit requires on-chain bridge interaction - not implemented yet".to_string(),
        ))
    }

    async fn withdraw(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        // Withdrawal from Hyperliquid L2
        let action = serde_json::json!({
            "type": "withdraw3",
            "hyperliquidChain": if self.testnet { "Testnet" } else { "Mainnet" },
            "signatureChainId": "0xa4b1",
            "destination": self.wallet.address_string(),
            "amount": _amount.to_string(),
            "time": chrono::Utc::now().timestamp_millis()
        });

        self.execute_action(action).await?;

        Ok(TxReceipt {
            tx_hash: String::new(),
            block_number: None,
            status: TxStatus::Pending,
            gas_used: None,
            gas_price: None,
            fee: None,
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
        })
    }
}
