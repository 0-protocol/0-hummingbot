//! EVM Wallet Implementation
//!
//! Provides wallet functionality for EVM-compatible chains using ethers-rs.

use ethers_core::types::{transaction::eip712::Eip712, Address, Signature, H256};
use ethers_signers::{LocalWallet, Signer};

use super::WalletError;

/// EVM wallet for signing transactions and messages
pub struct EvmWallet {
    /// The underlying wallet (private key)
    wallet: LocalWallet,
    /// RPC URL
    rpc_url: Option<String>,
    /// Chain ID
    chain_id: u64,
}

impl EvmWallet {
    /// Create a new wallet from a private key (hex string, with or without 0x prefix)
    pub fn from_private_key(private_key: &str, chain_id: u64) -> Result<Self, WalletError> {
        let key = private_key.strip_prefix("0x").unwrap_or(private_key);

        let wallet: LocalWallet = key
            .parse()
            .map_err(|e| WalletError::InvalidPrivateKey(format!("{}", e)))?;

        let wallet = wallet.with_chain_id(chain_id);

        Ok(Self {
            wallet,
            rpc_url: None,
            chain_id,
        })
    }

    /// Set the RPC URL
    pub fn with_rpc_url(mut self, rpc_url: &str) -> Self {
        self.rpc_url = Some(rpc_url.to_string());
        self
    }

    /// Get the wallet address
    pub fn address(&self) -> Address {
        self.wallet.address()
    }

    /// Get the checksummed address as a string
    pub fn address_string(&self) -> String {
        format!("{:?}", self.wallet.address())
    }

    /// Get the chain ID
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> Option<&str> {
        self.rpc_url.as_deref()
    }

    /// Sign a message (personal_sign / eth_sign format)
    pub async fn sign_message(&self, message: &[u8]) -> Result<Signature, WalletError> {
        self.wallet
            .sign_message(message)
            .await
            .map_err(|e| WalletError::SigningFailed(format!("{}", e)))
    }

    /// Sign a hash directly (without EIP-191 prefix)
    pub fn sign_hash(&self, hash: H256) -> Result<Signature, WalletError> {
        self.wallet
            .sign_hash(hash)
            .map_err(|e| WalletError::SigningFailed(format!("{}", e)))
    }

    /// Sign typed data (EIP-712)
    pub async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        typed_data: &T,
    ) -> Result<Signature, WalletError> {
        self.wallet
            .sign_typed_data(typed_data)
            .await
            .map_err(|e| WalletError::SigningFailed(format!("{}", e)))
    }

    /// Get the underlying wallet reference
    pub fn inner(&self) -> &LocalWallet {
        &self.wallet
    }
}

impl Clone for EvmWallet {
    fn clone(&self) -> Self {
        Self {
            wallet: self.wallet.clone(),
            rpc_url: self.rpc_url.clone(),
            chain_id: self.chain_id,
        }
    }
}

impl std::fmt::Debug for EvmWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvmWallet")
            .field("address", &self.address_string())
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_from_private_key() {
        // Test private key (DO NOT USE IN PRODUCTION)
        let test_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = EvmWallet::from_private_key(test_key, 1).unwrap();

        // This is the expected address for the test private key
        assert_eq!(
            wallet.address_string().to_lowercase(),
            "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
        );
    }

    #[test]
    fn test_wallet_from_key_without_prefix() {
        let test_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = EvmWallet::from_private_key(test_key, 42161).unwrap();

        assert_eq!(wallet.chain_id(), 42161);
    }

    #[tokio::test]
    async fn test_sign_message() {
        use ethers_core::types::U256;
        
        let test_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = EvmWallet::from_private_key(test_key, 1).unwrap();

        let message = b"Hello, Hyperliquid!";
        let signature = wallet.sign_message(message).await.unwrap();

        // Verify signature is non-empty
        assert_ne!(signature.r, U256::zero());
        assert_ne!(signature.s, U256::zero());
    }
}
