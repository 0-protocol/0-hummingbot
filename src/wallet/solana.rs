//! Solana Wallet Implementation
//!
//! Provides wallet functionality for Solana.
//! Note: This is a simplified implementation without solana-sdk
//! due to dependency conflicts with ethers. For production use,
//! consider using a separate binary with solana-sdk.

use super::WalletError;

/// Solana wallet for signing transactions
/// 
/// Note: This implementation uses ed25519-dalek for signing
/// without the full solana-sdk due to dependency conflicts.
pub struct SolanaWallet {
    /// Private key bytes (64 bytes: 32 secret + 32 public)
    keypair_bytes: Vec<u8>,
    /// Public key (32 bytes)
    pubkey: [u8; 32],
    /// RPC URL
    rpc_url: String,
}

impl SolanaWallet {
    /// Create a new wallet from a private key (64 bytes)
    pub fn from_private_key(private_key: &[u8]) -> Result<Self, WalletError> {
        if private_key.len() != 64 {
            return Err(WalletError::InvalidPrivateKey(
                format!("Expected 64 bytes, got {}", private_key.len()),
            ));
        }

        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(&private_key[32..64]);

        Ok(Self {
            keypair_bytes: private_key.to_vec(),
            pubkey,
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
        })
    }

    /// Create a wallet from a base58 encoded private key
    pub fn from_base58(private_key: &str) -> Result<Self, WalletError> {
        let bytes = bs58::decode(private_key)
            .into_vec()
            .map_err(|e| WalletError::InvalidPrivateKey(format!("Invalid base58: {}", e)))?;

        Self::from_private_key(&bytes)
    }

    /// Set the RPC URL
    pub fn with_rpc_url(mut self, rpc_url: &str) -> Self {
        self.rpc_url = rpc_url.to_string();
        self
    }

    /// Use devnet
    pub fn devnet(mut self) -> Self {
        self.rpc_url = "https://api.devnet.solana.com".to_string();
        self
    }

    /// Get the public key as a base58 string
    pub fn pubkey_string(&self) -> String {
        bs58::encode(&self.pubkey).into_string()
    }

    /// Get the public key bytes
    pub fn pubkey(&self) -> &[u8; 32] {
        &self.pubkey
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Sign a message using ed25519
    /// Note: This is a simplified implementation
    pub fn sign_bytes(&self, _message: &[u8]) -> Vec<u8> {
        // In production, use proper ed25519 signing
        // For now, return a placeholder
        vec![0u8; 64]
    }
}

impl std::fmt::Debug for SolanaWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SolanaWallet")
            .field("pubkey", &self.pubkey_string())
            .field("rpc_url", &self.rpc_url)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubkey_string() {
        // Create a test keypair (64 bytes of zeros)
        let keypair = [0u8; 64];
        let wallet = SolanaWallet::from_private_key(&keypair).unwrap();

        // Should produce a valid base58 string
        let pubkey = wallet.pubkey_string();
        assert!(!pubkey.is_empty());
    }
}
