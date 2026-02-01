//! Hyperliquid EIP-712 Signing

use ethers_core::types::{H256, U256};
use ethers_core::utils::keccak256;
use serde_json::Value;

use crate::connectors::ConnectorError;
use crate::wallet::EvmWallet;

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
    let action_str = serde_json::to_string(action)
        .map_err(|e| ConnectorError::Parse(e.to_string()))?;

    let mut hash_input = Vec::new();
    hash_input.extend_from_slice(action_str.as_bytes());
    hash_input.extend_from_slice(&nonce.to_le_bytes());
    hash_input.push(if is_mainnet { 1 } else { 0 });

    let action_hash = keccak256(&hash_input);
    
    let signature = wallet.sign_message(&action_hash).await
        .map_err(|e| ConnectorError::Auth(e.to_string()))?;

    Ok(format!("0x{:064x}{:064x}{:02x}", signature.r, signature.s, signature.v))
}
