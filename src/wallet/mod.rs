//! Wallet implementations for different blockchain networks
//!
//! This module provides wallet functionality for signing transactions
//! across different blockchain networks (EVM, Cosmos, Solana).

pub mod cosmos;
pub mod evm;
pub mod solana;

use crate::connectors::ConnectorError;

/// Common wallet error type
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),

    #[error("Nonce error: {0}")]
    NonceError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<WalletError> for ConnectorError {
    fn from(err: WalletError) -> Self {
        ConnectorError::WalletError(err.to_string())
    }
}

// Re-export wallet types
pub use cosmos::CosmosWallet;
pub use evm::EvmWallet;
pub use solana::SolanaWallet;
