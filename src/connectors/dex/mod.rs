//! DEX (Decentralized Exchange) Connectors
//!
//! This module provides connectors for decentralized exchanges, extending
//! the base ConnectorBase trait with DEX-specific functionality like
//! wallet management and on-chain transaction handling.

pub mod dydx;
pub mod hyperliquid;
pub mod jupiter;

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::connectors::{ConnectorBase, ConnectorError, TxReceipt};

/// DEX-specific extension trait
///
/// Extends ConnectorBase with functionality specific to decentralized exchanges,
/// including wallet operations and on-chain transaction handling.
#[async_trait]
pub trait DexConnector: ConnectorBase {
    /// Get the wallet address
    fn wallet_address(&self) -> &str;

    /// Get the chain ID
    fn chain_id(&self) -> u64;

    /// Sign a message with the wallet
    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ConnectorError>;

    /// Sign typed data (EIP-712 for EVM chains)
    async fn sign_typed_data(&self, typed_data: &str) -> Result<Vec<u8>, ConnectorError>;

    /// Get on-chain gas estimate for an action
    async fn estimate_gas(&self, action: &str) -> Result<Decimal, ConnectorError>;

    /// Wait for transaction confirmation
    async fn wait_for_confirmation(
        &self,
        tx_hash: &str,
        confirmations: u32,
    ) -> Result<TxReceipt, ConnectorError>;

    /// Get transaction receipt
    async fn get_tx_receipt(&self, tx_hash: &str) -> Result<TxReceipt, ConnectorError>;

    /// Approve token spending (for EVM DEXes)
    async fn approve_token(
        &self,
        token_address: &str,
        spender: &str,
        amount: Decimal,
    ) -> Result<TxReceipt, ConnectorError>;

    /// Check token allowance
    async fn check_allowance(
        &self,
        token_address: &str,
        spender: &str,
    ) -> Result<Decimal, ConnectorError>;

    /// Deposit funds to the DEX (if applicable)
    async fn deposit(&self, asset: &str, amount: Decimal) -> Result<TxReceipt, ConnectorError>;

    /// Withdraw funds from the DEX
    async fn withdraw(&self, asset: &str, amount: Decimal) -> Result<TxReceipt, ConnectorError>;
}

/// Chain type for DEX connectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainType {
    /// EVM-compatible chains (Ethereum, Arbitrum, etc.)
    Evm,
    /// Cosmos SDK chains
    Cosmos,
    /// Solana
    Solana,
}

impl std::fmt::Display for ChainType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainType::Evm => write!(f, "EVM"),
            ChainType::Cosmos => write!(f, "Cosmos"),
            ChainType::Solana => write!(f, "Solana"),
        }
    }
}

// Re-export connectors
pub use dydx::DydxConnector;
pub use hyperliquid::HyperliquidConnector;
pub use jupiter::JupiterConnector;
