//! Jupiter Connector
//!
//! Jupiter is the leading swap aggregator on Solana.
//! This connector implements the Jupiter API for token swaps.

mod api;
mod signing;

pub use api::JupiterApi;
pub use signing::*;

use async_trait::async_trait;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::connectors::dex::DexConnector;
use crate::connectors::{
    Balance, ConnectorBase, ConnectorError, ExchangeType, Order, OrderBook, OrderBookLevel,
    OrderRequest, OrderResponse, OrderSide, OrderStatus, Position, Ticker, Trade, TradingPair,
    TxReceipt, TxStatus,
};
use crate::wallet::SolanaWallet;

/// Jupiter API endpoints
pub const JUPITER_API_URL: &str = "https://quote-api.jup.ag/v6";
pub const JUPITER_PRICE_API_URL: &str = "https://price.jup.ag/v6";

/// Common Solana token mints
pub mod tokens {
    pub const SOL: &str = "So11111111111111111111111111111111111111112";
    pub const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    pub const USDT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
    pub const BONK: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
    pub const JUP: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";
}

/// Jupiter connector for Solana swaps
pub struct JupiterConnector {
    /// Solana wallet for signing
    wallet: SolanaWallet,
    /// HTTP client
    client: reqwest::Client,
    /// Quote API URL
    api_url: String,
    /// Price API URL
    price_api_url: String,
    /// Connection status
    connected: Arc<RwLock<bool>>,
    /// Slippage tolerance (basis points)
    slippage_bps: u32,
}

impl JupiterConnector {
    /// Create a new Jupiter connector
    pub fn new(private_key: &[u8]) -> Result<Self, ConnectorError> {
        let wallet = SolanaWallet::from_private_key(private_key)
            .map_err(|e| ConnectorError::WalletError(e.to_string()))?;

        Ok(Self {
            wallet,
            client: reqwest::Client::new(),
            api_url: JUPITER_API_URL.to_string(),
            price_api_url: JUPITER_PRICE_API_URL.to_string(),
            connected: Arc::new(RwLock::new(false)),
            slippage_bps: 50, // 0.5% default slippage
        })
    }

    /// Create from base58 private key
    pub fn from_base58(private_key: &str) -> Result<Self, ConnectorError> {
        let wallet = SolanaWallet::from_base58(private_key)
            .map_err(|e| ConnectorError::WalletError(e.to_string()))?;

        Ok(Self {
            wallet,
            client: reqwest::Client::new(),
            api_url: JUPITER_API_URL.to_string(),
            price_api_url: JUPITER_PRICE_API_URL.to_string(),
            connected: Arc::new(RwLock::new(false)),
            slippage_bps: 50,
        })
    }

    /// Use devnet
    pub fn devnet(mut self) -> Self {
        self.wallet = self.wallet.devnet();
        self
    }

    /// Set slippage tolerance
    pub fn with_slippage(mut self, slippage_bps: u32) -> Self {
        self.slippage_bps = slippage_bps;
        self
    }

    /// Get quote for a swap
    pub async fn get_quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: u64,
    ) -> Result<JupiterQuote, ConnectorError> {
        let url = format!(
            "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps={}",
            self.api_url, input_mint, output_mint, amount, self.slippage_bps
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(ConnectorError::InvalidResponse(error_text));
        }

        let quote: JupiterQuote = resp.json().await?;
        Ok(quote)
    }

    /// Get swap transaction
    pub async fn get_swap_transaction(
        &self,
        quote: &JupiterQuote,
    ) -> Result<String, ConnectorError> {
        let body = serde_json::json!({
            "quoteResponse": quote,
            "userPublicKey": self.wallet.pubkey_string(),
            "wrapAndUnwrapSol": true,
            "dynamicComputeUnitLimit": true,
            "prioritizationFeeLamports": "auto"
        });

        let resp = self
            .client
            .post(&format!("{}/swap", self.api_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(ConnectorError::InvalidResponse(error_text));
        }

        let swap_response: JupiterSwapResponse = resp.json().await?;
        Ok(swap_response.swap_transaction)
    }

    /// Get token price
    pub async fn get_price(&self, mint: &str) -> Result<Decimal, ConnectorError> {
        let url = format!("{}/price?ids={}", self.price_api_url, mint);

        let resp = self.client.get(&url).send().await?;
        let data: JupiterPriceResponse = resp.json().await?;

        data.data
            .get(mint)
            .and_then(|p| p.price.parse().ok())
            .ok_or_else(|| ConnectorError::NotFound(format!("Price not found for {}", mint)))
    }

    /// Convert pair string to mints
    fn parse_pair(&self, pair: &str) -> Result<(String, String), ConnectorError> {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            return Err(ConnectorError::InvalidResponse(format!(
                "Invalid pair format: {}",
                pair
            )));
        }

        let input_mint = self.symbol_to_mint(parts[0])?;
        let output_mint = self.symbol_to_mint(parts[1])?;

        Ok((input_mint, output_mint))
    }

    /// Convert symbol to mint address
    fn symbol_to_mint(&self, symbol: &str) -> Result<String, ConnectorError> {
        match symbol.to_uppercase().as_str() {
            "SOL" => Ok(tokens::SOL.to_string()),
            "USDC" => Ok(tokens::USDC.to_string()),
            "USDT" => Ok(tokens::USDT.to_string()),
            "BONK" => Ok(tokens::BONK.to_string()),
            "JUP" => Ok(tokens::JUP.to_string()),
            // If it looks like a mint address, return it directly
            s if s.len() > 30 => Ok(s.to_string()),
            _ => Err(ConnectorError::NotFound(format!(
                "Unknown token symbol: {}",
                symbol
            ))),
        }
    }
}

#[async_trait]
impl ConnectorBase for JupiterConnector {
    fn name(&self) -> &str {
        "jupiter"
    }

    fn exchange_type(&self) -> ExchangeType {
        ExchangeType::Swap
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn connect(&mut self) -> Result<(), ConnectorError> {
        // Test connection by getting SOL price
        let resp = self
            .client
            .get(&format!("{}/price?ids={}", self.price_api_url, tokens::SOL))
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
        let (input_mint, output_mint) = self.parse_pair(pair)?;

        let input_price = self.get_price(&input_mint).await?;
        let output_price = self.get_price(&output_mint).await?;

        let price = if output_price != Decimal::ZERO {
            input_price / output_price
        } else {
            Decimal::ZERO
        };

        Ok(Ticker {
            pair: pair.to_string(),
            last_price: price,
            bid: price, // Jupiter is an aggregator - no bid/ask
            ask: price,
            high_24h: Decimal::ZERO,
            low_24h: Decimal::ZERO,
            volume_24h: Decimal::ZERO,
            change_24h: Decimal::ZERO,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn get_orderbook(&self, _pair: &str, _depth: u32) -> Result<OrderBook, ConnectorError> {
        // Jupiter is an aggregator - no traditional orderbook
        Err(ConnectorError::Internal(
            "Jupiter is a swap aggregator - no orderbook available".to_string(),
        ))
    }

    async fn get_trades(&self, _pair: &str, _limit: u32) -> Result<Vec<Trade>, ConnectorError> {
        // Jupiter doesn't provide trade history
        Ok(vec![])
    }

    async fn get_pairs(&self) -> Result<Vec<TradingPair>, ConnectorError> {
        // Return common trading pairs
        let pairs = vec![
            ("SOL/USDC", tokens::SOL, tokens::USDC),
            ("JUP/USDC", tokens::JUP, tokens::USDC),
            ("BONK/USDC", tokens::BONK, tokens::USDC),
            ("SOL/USDT", tokens::SOL, tokens::USDT),
        ];

        Ok(pairs
            .into_iter()
            .map(|(symbol, base, _quote)| TradingPair {
                symbol: symbol.to_string(),
                base: base.to_string(),
                quote: "USDC".to_string(),
                min_order_size: Decimal::new(1, 9), // 1 lamport
                tick_size: Decimal::new(1, 6),
                step_size: Decimal::new(1, 9),
                is_active: true,
            })
            .collect())
    }

    async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse, ConnectorError> {
        // Jupiter swap = market order
        let (input_mint, output_mint) = self.parse_pair(&order.pair)?;

        // Convert quantity to lamports/atomic units
        let amount = (order.quantity * Decimal::new(1_000_000_000, 0))
            .to_string()
            .parse::<u64>()
            .map_err(|_| ConnectorError::InvalidResponse("Invalid amount".to_string()))?;

        // Get quote
        let quote = match order.side {
            OrderSide::Buy => self.get_quote(&output_mint, &input_mint, amount).await?,
            OrderSide::Sell => self.get_quote(&input_mint, &output_mint, amount).await?,
        };

        // Get swap transaction
        let swap_tx = self.get_swap_transaction(&quote).await?;

        // In production, we would:
        // 1. Deserialize the transaction
        // 2. Sign it with the wallet
        // 3. Send it to Solana RPC
        // For now, return the transaction for manual execution

        Ok(OrderResponse {
            order_id: swap_tx.clone(), // Use TX as order ID
            client_order_id: order.client_order_id.clone(),
            status: OrderStatus::Pending,
            filled_quantity: Decimal::ZERO,
            avg_fill_price: Some(
                quote
                    .out_amount
                    .parse::<Decimal>()
                    .unwrap_or_default()
                    / quote.in_amount.parse::<Decimal>().unwrap_or(Decimal::ONE),
            ),
            tx_hash: Some(swap_tx),
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }

    async fn cancel_order(&self, _order_id: &str, _pair: &str) -> Result<(), ConnectorError> {
        // Swaps are atomic - can't be canceled
        Err(ConnectorError::Internal(
            "Jupiter swaps are atomic and cannot be canceled".to_string(),
        ))
    }

    async fn get_order(&self, _order_id: &str, _pair: &str) -> Result<Order, ConnectorError> {
        // Would need to query Solana for TX status
        Err(ConnectorError::Internal(
            "Query Solana RPC for transaction status".to_string(),
        ))
    }

    async fn get_open_orders(&self, _pair: Option<&str>) -> Result<Vec<Order>, ConnectorError> {
        // No open orders for swaps
        Ok(vec![])
    }

    async fn get_balances(&self) -> Result<Vec<Balance>, ConnectorError> {
        // Would need to query Solana RPC for token balances
        // For now, return empty
        Ok(vec![])
    }

    async fn get_positions(&self) -> Result<Vec<Position>, ConnectorError> {
        // No positions for swaps
        Ok(vec![])
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
impl DexConnector for JupiterConnector {
    fn wallet_address(&self) -> &str {
        Box::leak(self.wallet.pubkey_string().into_boxed_str())
    }

    fn chain_id(&self) -> u64 {
        // Solana doesn't use numeric chain IDs
        0
    }

    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ConnectorError> {
        Ok(self.wallet.sign_bytes(message))
    }

    async fn sign_typed_data(&self, _typed_data: &str) -> Result<Vec<u8>, ConnectorError> {
        Err(ConnectorError::Internal(
            "Solana does not use EIP-712 typed data".to_string(),
        ))
    }

    async fn estimate_gas(&self, _action: &str) -> Result<Decimal, ConnectorError> {
        // Solana transaction fee estimation
        // Base fee is typically 5000 lamports + priority fee
        Ok(Decimal::new(10000, 9)) // 0.00001 SOL
    }

    async fn wait_for_confirmation(
        &self,
        tx_hash: &str,
        _confirmations: u32,
    ) -> Result<TxReceipt, ConnectorError> {
        // Would query Solana RPC for confirmation
        Ok(TxReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: None,
            status: TxStatus::Pending,
            gas_used: None,
            gas_price: None,
            fee: Some(Decimal::new(5000, 9)), // 5000 lamports
            timestamp: Some(chrono::Utc::now().timestamp_millis()),
        })
    }

    async fn get_tx_receipt(&self, tx_hash: &str) -> Result<TxReceipt, ConnectorError> {
        Ok(TxReceipt {
            tx_hash: tx_hash.to_string(),
            block_number: None,
            status: TxStatus::Pending,
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
        // Solana uses token accounts, not approvals
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
        // Solana doesn't use allowances
        Ok(Decimal::MAX)
    }

    async fn deposit(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        // No deposits needed for Jupiter
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

    async fn withdraw(&self, _asset: &str, _amount: Decimal) -> Result<TxReceipt, ConnectorError> {
        // No withdrawals needed for Jupiter
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
}

// Response types for Jupiter API
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterQuote {
    pub input_mint: String,
    pub in_amount: String,
    pub output_mint: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u32,
    pub price_impact_pct: String,
    pub route_plan: Vec<RoutePlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: Option<String>,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: String,
    pub fee_mint: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterSwapResponse {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
    pub prioritization_fee_lamports: u64,
}

#[derive(Debug, Deserialize)]
pub struct JupiterPriceResponse {
    pub data: std::collections::HashMap<String, JupiterPrice>,
}

#[derive(Debug, Deserialize)]
pub struct JupiterPrice {
    pub id: String,
    #[serde(rename = "mintSymbol")]
    pub mint_symbol: String,
    #[serde(rename = "vsToken")]
    pub vs_token: String,
    #[serde(rename = "vsTokenSymbol")]
    pub vs_token_symbol: String,
    pub price: String,
}
