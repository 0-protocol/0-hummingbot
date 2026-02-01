//! Hyperliquid EIP-712 Signing
//!
//! Implements EIP-712 typed data signing for Hyperliquid actions.
//! Hyperliquid uses EIP-712 for order signing on their L2.

use ethers_core::types::transaction::eip712::{EIP712Domain, Eip712};
use ethers_core::types::{H160, H256, U256};
use ethers_core::utils::keccak256;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::connectors::ConnectorError;
use crate::wallet::EvmWallet;

/// Hyperliquid L1 action signing domain
const DOMAIN_NAME: &str = "HyperliquidSignTransaction";
const DOMAIN_VERSION: &str = "1";

/// Phantom agent for signing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhantomAgent {
    pub source: String,
    #[serde(rename = "connectionId")]
    pub connection_id: H256,
}

impl Eip712 for PhantomAgent {
    type Error = String;

    fn domain(&self) -> Result<EIP712Domain, Self::Error> {
        Ok(EIP712Domain {
            name: Some(DOMAIN_NAME.to_string()),
            version: Some(DOMAIN_VERSION.to_string()),
            chain_id: Some(U256::from(42161)), // Arbitrum
            verifying_contract: None,
            salt: None,
        })
    }

    fn type_hash() -> Result<[u8; 32], Self::Error> {
        // Agent(string source,bytes32 connectionId)
        let type_string = "Agent(string source,bytes32 connectionId)";
        let hash = keccak256(type_string.as_bytes());
        Ok(hash)
    }

    fn struct_hash(&self) -> Result<[u8; 32], Self::Error> {
        // Encode: keccak256(typeHash, keccak256(source), connectionId)
        let type_hash = Self::type_hash()?;
        let source_hash = keccak256(self.source.as_bytes());

        let mut encoded = Vec::new();
        encoded.extend_from_slice(&type_hash);
        encoded.extend_from_slice(&source_hash);
        encoded.extend_from_slice(self.connection_id.as_bytes());

        Ok(keccak256(&encoded))
    }
}

/// Build an order action for signing
pub fn build_order_action(
    asset_index: u32,
    is_buy: bool,
    price: &str,
    size: &str,
    order_type: Value,
    reduce_only: bool,
) -> Value {
    serde_json::json!({
        "type": "order",
        "orders": [{
            "a": asset_index,
            "b": is_buy,
            "p": price,
            "s": size,
            "r": reduce_only,
            "t": order_type
        }],
        "grouping": "na"
    })
}

/// Sign an L1 action for Hyperliquid
pub async fn sign_l1_action(
    wallet: &EvmWallet,
    action: &Value,
    nonce: u64,
    is_mainnet: bool,
) -> Result<String, ConnectorError> {
    // Hyperliquid uses a specific signing scheme:
    // 1. Hash the action with keccak256
    // 2. Create a phantom agent with the action hash as connectionId
    // 3. Sign the EIP-712 typed data
    
    // Step 1: Create action hash
    let action_str = serde_json::to_string(action)
        .map_err(|e| ConnectorError::Internal(format!("Failed to serialize action: {}", e)))?;

    // Include nonce and mainnet flag in the hash
    let mut hash_input = Vec::new();
    hash_input.extend_from_slice(action_str.as_bytes());
    hash_input.extend_from_slice(&nonce.to_le_bytes());
    hash_input.push(if is_mainnet { 1 } else { 0 });

    let action_hash = keccak256(&hash_input);

    // Step 2: Create phantom agent
    let phantom_agent = PhantomAgent {
        source: if is_mainnet { "a".to_string() } else { "b".to_string() },
        connection_id: H256::from_slice(&action_hash),
    };

    // Step 3: Sign the typed data
    let signature = wallet
        .sign_typed_data(&phantom_agent)
        .await
        .map_err(|e| ConnectorError::SigningError(format!("Failed to sign: {}", e)))?;

    // Format signature as hex string with proper recovery ID
    let r = format!("{:064x}", signature.r);
    let s = format!("{:064x}", signature.s);
    let v = signature.v;

    Ok(format!("0x{}{}{:02x}", r, s, v))
}

/// Verify a signature (for testing)
pub fn verify_signature(
    address: &str,
    message: &[u8],
    signature: &str,
) -> Result<bool, ConnectorError> {
    use ethers_core::types::Signature;

    let sig = signature
        .strip_prefix("0x")
        .unwrap_or(signature);

    if sig.len() != 130 {
        return Err(ConnectorError::InvalidResponse(
            "Invalid signature length".to_string(),
        ));
    }

    let r = U256::from_str_radix(&sig[0..64], 16)
        .map_err(|_| ConnectorError::InvalidResponse("Invalid r".to_string()))?;
    let s = U256::from_str_radix(&sig[64..128], 16)
        .map_err(|_| ConnectorError::InvalidResponse("Invalid s".to_string()))?;
    let v = u64::from_str_radix(&sig[128..130], 16)
        .map_err(|_| ConnectorError::InvalidResponse("Invalid v".to_string()))?;

    let signature = Signature { r, s, v };

    let recovered = signature
        .recover(message)
        .map_err(|e| ConnectorError::SigningError(format!("Recovery failed: {}", e)))?;

    let expected: H160 = address
        .parse()
        .map_err(|_| ConnectorError::InvalidResponse("Invalid address".to_string()))?;

    Ok(recovered == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_order_action() {
        let action = build_order_action(
            0,
            true,
            "50000.0",
            "0.1",
            serde_json::json!({"limit": {"tif": "Gtc"}}),
            false,
        );

        assert_eq!(action["type"], "order");
        assert!(action["orders"].is_array());

        let order = &action["orders"][0];
        assert_eq!(order["a"], 0);
        assert_eq!(order["b"], true);
        assert_eq!(order["p"], "50000.0");
        assert_eq!(order["s"], "0.1");
    }

    #[tokio::test]
    async fn test_sign_l1_action() {
        // Test private key (DO NOT USE IN PRODUCTION)
        let test_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = EvmWallet::from_private_key(test_key, 42161).unwrap();

        let action = build_order_action(
            0,
            true,
            "50000.0",
            "0.1",
            serde_json::json!({"limit": {"tif": "Gtc"}}),
            false,
        );

        let nonce = 1234567890u64;
        let signature = sign_l1_action(&wallet, &action, nonce, true).await.unwrap();

        // Signature should be 132 characters (0x + 64 + 64 + 2)
        assert!(signature.starts_with("0x"));
        assert_eq!(signature.len(), 132);
    }
}
