//! Jupiter Transaction Signing
//!
//! Utilities for signing and submitting Jupiter swap transactions on Solana.

use crate::connectors::ConnectorError;
use crate::wallet::SolanaWallet;

/// Sign a serialized transaction
pub fn sign_transaction(
    wallet: &SolanaWallet,
    transaction_bytes: &[u8],
) -> Result<Vec<u8>, ConnectorError> {
    // In production, this would:
    // 1. Deserialize the transaction from base64
    // 2. Sign with the wallet keypair
    // 3. Return the signed transaction bytes

    // For now, return the signature of the transaction bytes
    Ok(wallet.sign_bytes(transaction_bytes))
}

/// Decode base64 transaction
pub fn decode_transaction(base64_tx: &str) -> Result<Vec<u8>, ConnectorError> {
    use base64::{engine::general_purpose, Engine};

    general_purpose::STANDARD
        .decode(base64_tx)
        .map_err(|e| ConnectorError::InvalidResponse(format!("Invalid base64: {}", e)))
}

/// Encode transaction to base64
pub fn encode_transaction(tx_bytes: &[u8]) -> String {
    use base64::{engine::general_purpose, Engine};

    general_purpose::STANDARD.encode(tx_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let original = b"test transaction data";
        let encoded = encode_transaction(original);
        let decoded = decode_transaction(&encoded).unwrap();

        assert_eq!(original.as_slice(), decoded.as_slice());
    }
}
