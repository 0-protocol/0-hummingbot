//! Cosmos Wallet Implementation
//!
//! Provides wallet functionality for Cosmos SDK chains.
//! Note: This is a basic implementation. For production use with dYdX v4,
//! consider using the official dYdX client library.

use super::WalletError;
use sha2::{Digest, Sha256};

/// Cosmos wallet for signing transactions
pub struct CosmosWallet {
    /// Private key bytes (32 bytes)
    private_key: Vec<u8>,
    /// Public key bytes (33 bytes compressed)
    public_key: Vec<u8>,
    /// Bech32 address prefix
    address_prefix: String,
    /// RPC URL
    rpc_url: String,
}

impl CosmosWallet {
    /// Create a new wallet from a private key (hex encoded)
    pub fn from_private_key(private_key: &str, address_prefix: &str) -> Result<Self, WalletError> {
        let key = private_key.strip_prefix("0x").unwrap_or(private_key);

        let private_key_bytes = hex::decode(key)
            .map_err(|e| WalletError::InvalidPrivateKey(format!("Invalid hex: {}", e)))?;

        if private_key_bytes.len() != 32 {
            return Err(WalletError::InvalidPrivateKey(
                "Private key must be 32 bytes".to_string(),
            ));
        }

        // Derive public key using k256 (same curve as secp256k1)
        use k256::ecdsa::SigningKey;
        let signing_key = SigningKey::from_slice(&private_key_bytes)
            .map_err(|e| WalletError::InvalidPrivateKey(format!("{}", e)))?;

        let verifying_key = signing_key.verifying_key();
        let public_key_bytes = verifying_key.to_encoded_point(true).as_bytes().to_vec();

        Ok(Self {
            private_key: private_key_bytes,
            public_key: public_key_bytes,
            address_prefix: address_prefix.to_string(),
            rpc_url: "https://dydx-mainnet-full-rpc.public.blastapi.io".to_string(),
        })
    }

    /// Create a wallet for dYdX v4
    pub fn for_dydx(private_key: &str) -> Result<Self, WalletError> {
        Self::from_private_key(private_key, "dydx")
    }

    /// Set the RPC URL
    pub fn with_rpc_url(mut self, rpc_url: &str) -> Self {
        self.rpc_url = rpc_url.to_string();
        self
    }

    /// Get the bech32 address
    pub fn address(&self) -> Result<String, WalletError> {
        // Hash the public key with SHA256
        let sha_hash = Sha256::digest(&self.public_key);

        // Take first 20 bytes (RIPEMD160 equivalent for cosmos)
        let address_bytes = &sha_hash[..20];

        // Encode as bech32
        bech32::encode(&self.address_prefix, address_bytes.to_vec(), bech32::Variant::Bech32)
            .map_err(|e| WalletError::InvalidAddress(format!("Bech32 encoding failed: {}", e)))
    }

    /// Get the public key bytes
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Sign a message (returns signature bytes)
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, WalletError> {
        use k256::ecdsa::{signature::Signer, Signature, SigningKey};

        // Hash the message
        let message_hash = Sha256::digest(message);

        // Sign
        let signing_key = SigningKey::from_slice(&self.private_key)
            .map_err(|e| WalletError::SigningFailed(format!("Invalid key: {}", e)))?;

        let signature: Signature = signing_key.sign(&message_hash);

        Ok(signature.to_bytes().to_vec())
    }

    /// Sign a Cosmos transaction (simplified)
    pub fn sign_tx(&self, sign_doc: &[u8]) -> Result<Vec<u8>, WalletError> {
        self.sign(sign_doc)
    }
}

impl std::fmt::Debug for CosmosWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CosmosWallet")
            .field("address_prefix", &self.address_prefix)
            .field("rpc_url", &self.rpc_url)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        // Test private key (DO NOT USE IN PRODUCTION)
        let test_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = CosmosWallet::for_dydx(test_key).unwrap();

        let address = wallet.address().unwrap();
        assert!(address.starts_with("dydx"));
    }

    #[test]
    fn test_sign_message() {
        let test_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = CosmosWallet::for_dydx(test_key).unwrap();

        let message = b"Hello, dYdX!";
        let signature = wallet.sign(message).unwrap();

        assert_eq!(signature.len(), 64); // Compact signature is 64 bytes
    }
}
