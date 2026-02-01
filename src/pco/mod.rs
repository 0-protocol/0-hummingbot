//! Proof-Carrying Orders (PCO) System
//!
//! A Proof-Carrying Order contains cryptographic proof of strategy intent,
//! linking every order to the strategy graph and market data that generated it.
//!
//! This enables:
//! - Verifiable strategy execution
//! - Audit trails for compliance
//! - Agent accountability
//! - Deterministic replay

mod builder;
mod verifier;

pub use builder::PcoBuilder;
pub use verifier::PcoVerifier;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Error types for PCO operations
#[derive(Debug, thiserror::Error)]
pub enum PcoError {
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Execution trace verification failed: {0}")]
    InvalidExecutionTrace(String),

    #[error("Strategy hash mismatch")]
    StrategyHashMismatch,

    #[error("Input hash mismatch")]
    InputHashMismatch,

    #[error("Timestamp validation failed: {0}")]
    TimestampError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    TakeProfit,
}

/// A trading order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Trading pair symbol
    pub symbol: String,

    /// Order side (buy/sell)
    pub side: OrderSide,

    /// Order type
    pub order_type: OrderType,

    /// Order quantity
    pub quantity: f64,

    /// Limit price (for limit orders)
    pub price: Option<f64>,

    /// Stop price (for stop orders)
    pub stop_price: Option<f64>,

    /// Time-in-force
    pub time_in_force: String,

    /// Client order ID
    pub client_order_id: String,
}

impl Order {
    /// Serialize order to bytes for signing
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Symbol
        bytes.extend_from_slice(self.symbol.as_bytes());
        bytes.push(0); // null separator

        // Side
        bytes.push(match self.side {
            OrderSide::Buy => 0,
            OrderSide::Sell => 1,
        });

        // Order type
        bytes.push(match self.order_type {
            OrderType::Market => 0,
            OrderType::Limit => 1,
            OrderType::StopLoss => 2,
            OrderType::TakeProfit => 3,
        });

        // Quantity (as f64 bytes)
        bytes.extend_from_slice(&self.quantity.to_le_bytes());

        // Price (optional)
        if let Some(price) = self.price {
            bytes.push(1);
            bytes.extend_from_slice(&price.to_le_bytes());
        } else {
            bytes.push(0);
        }

        // Stop price (optional)
        if let Some(stop_price) = self.stop_price {
            bytes.push(1);
            bytes.extend_from_slice(&stop_price.to_le_bytes());
        } else {
            bytes.push(0);
        }

        // Time in force
        bytes.extend_from_slice(self.time_in_force.as_bytes());
        bytes.push(0);

        // Client order ID
        bytes.extend_from_slice(self.client_order_id.as_bytes());

        bytes
    }
}

/// A Proof-Carrying Order contains cryptographic proof of strategy intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCarryingOrder {
    /// The actual order to execute
    pub order: Order,

    /// Hash of the strategy graph that generated this order
    pub strategy_hash: [u8; 32],

    /// Hash of the market data inputs at decision time
    pub input_hash: [u8; 32],

    /// Execution trace: hashes of all nodes evaluated
    pub execution_trace: Vec<[u8; 32]>,

    /// Timestamp of order generation (Unix timestamp)
    pub timestamp: u64,

    /// Agent's public key
    pub agent_pubkey: [u8; 32],

    /// Ed25519 signature over the above fields
    pub signature: [u8; 64],
}

impl ProofCarryingOrder {
    /// Create a new PCO from order and execution context
    pub fn new(
        order: Order,
        strategy_hash: [u8; 32],
        input_hash: [u8; 32],
        execution_trace: Vec<[u8; 32]>,
        signing_key: &SigningKey,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let agent_pubkey = signing_key.verifying_key().to_bytes();

        // Create message to sign
        let message = Self::build_message_static(
            &order,
            &strategy_hash,
            &input_hash,
            &execution_trace,
            timestamp,
            &agent_pubkey,
        );

        let signature = signing_key.sign(&message);

        Self {
            order,
            strategy_hash,
            input_hash,
            execution_trace,
            timestamp,
            agent_pubkey,
            signature: signature.to_bytes(),
        }
    }

    /// Build the message bytes for signing/verification
    fn build_message_static(
        order: &Order,
        strategy_hash: &[u8; 32],
        input_hash: &[u8; 32],
        execution_trace: &[[u8; 32]],
        timestamp: u64,
        agent_pubkey: &[u8; 32],
    ) -> Vec<u8> {
        let mut message = Vec::new();

        // Order bytes
        message.extend_from_slice(&order.to_bytes());

        // Strategy hash
        message.extend_from_slice(strategy_hash);

        // Input hash
        message.extend_from_slice(input_hash);

        // Execution trace
        for trace in execution_trace {
            message.extend_from_slice(trace);
        }

        // Timestamp
        message.extend_from_slice(&timestamp.to_le_bytes());

        // Agent public key
        message.extend_from_slice(agent_pubkey);

        message
    }

    /// Build the message bytes for this PCO
    fn build_message(&self) -> Vec<u8> {
        Self::build_message_static(
            &self.order,
            &self.strategy_hash,
            &self.input_hash,
            &self.execution_trace,
            self.timestamp,
            &self.agent_pubkey,
        )
    }

    /// Verify the PCO signature
    pub fn verify_signature(&self) -> Result<bool, PcoError> {
        let public_key = VerifyingKey::from_bytes(&self.agent_pubkey)
            .map_err(|e| PcoError::InvalidPublicKey(e.to_string()))?;

        let signature = Signature::from_bytes(&self.signature);
        let message = self.build_message();

        public_key
            .verify(&message, &signature)
            .map(|_| true)
            .map_err(|e| PcoError::InvalidSignature(e.to_string()))
    }

    /// Verify that the timestamp is within acceptable bounds
    pub fn verify_timestamp(&self, max_age_secs: u64) -> Result<bool, PcoError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        // Check if timestamp is not in the future (with small tolerance)
        if self.timestamp > now + 60 {
            return Err(PcoError::TimestampError("Timestamp is in the future".into()));
        }

        // Check if timestamp is not too old
        if now - self.timestamp > max_age_secs {
            return Err(PcoError::TimestampError(format!(
                "Timestamp is too old: {} seconds ago",
                now - self.timestamp
            )));
        }

        Ok(true)
    }

    /// Get a summary of the execution trace
    pub fn trace_summary(&self) -> String {
        format!(
            "PCO Trace: {} nodes evaluated, strategy={}, timestamp={}",
            self.execution_trace.len(),
            hex::encode(&self.strategy_hash[..8]),
            self.timestamp
        )
    }

    /// Hash the entire PCO for reference
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.order.to_bytes());
        hasher.update(&self.strategy_hash);
        hasher.update(&self.input_hash);
        for trace in &self.execution_trace {
            hasher.update(trace);
        }
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(&self.agent_pubkey);
        hasher.update(&self.signature);

        hasher.finalize().into()
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, PcoError> {
        serde_json::to_string_pretty(self).map_err(|e| PcoError::SerializationError(e.to_string()))
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, PcoError> {
        serde_json::from_str(json).map_err(|e| PcoError::SerializationError(e.to_string()))
    }
}

/// Market data snapshot used as input to strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataInput {
    /// Timestamp of the snapshot
    pub timestamp: u64,

    /// Symbol
    pub symbol: String,

    /// Best bid price
    pub best_bid: f64,

    /// Best ask price
    pub best_ask: f64,

    /// Best bid size
    pub bid_size: f64,

    /// Best ask size
    pub ask_size: f64,

    /// Mid price
    pub mid_price: f64,

    /// Recent volatility
    pub volatility: Option<f64>,

    /// Current position
    pub position: Option<f64>,
}

impl MarketDataInput {
    /// Hash the market data for input_hash field
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(self.symbol.as_bytes());
        hasher.update(&self.best_bid.to_le_bytes());
        hasher.update(&self.best_ask.to_le_bytes());
        hasher.update(&self.bid_size.to_le_bytes());
        hasher.update(&self.ask_size.to_le_bytes());
        hasher.update(&self.mid_price.to_le_bytes());

        if let Some(vol) = self.volatility {
            hasher.update(&[1u8]);
            hasher.update(&vol.to_le_bytes());
        } else {
            hasher.update(&[0u8]);
        }

        if let Some(pos) = self.position {
            hasher.update(&[1u8]);
            hasher.update(&pos.to_le_bytes());
        } else {
            hasher.update(&[0u8]);
        }

        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn create_test_order() -> Order {
        Order {
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity: 0.01,
            price: Some(50000.0),
            stop_price: None,
            time_in_force: "GTC".to_string(),
            client_order_id: "test-001".to_string(),
        }
    }

    #[test]
    fn test_pco_creation_and_verification() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let order = create_test_order();

        let strategy_hash = [1u8; 32];
        let input_hash = [2u8; 32];
        let execution_trace = vec![[3u8; 32], [4u8; 32], [5u8; 32]];

        let pco = ProofCarryingOrder::new(
            order,
            strategy_hash,
            input_hash,
            execution_trace,
            &signing_key,
        );

        // Verify signature
        assert!(pco.verify_signature().unwrap());

        // Verify timestamp (should be recent)
        assert!(pco.verify_timestamp(60).unwrap());
    }

    #[test]
    fn test_pco_tamper_detection() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let order = create_test_order();

        let pco = ProofCarryingOrder::new(
            order,
            [1u8; 32],
            [2u8; 32],
            vec![[3u8; 32]],
            &signing_key,
        );

        // Tamper with the strategy hash
        let mut tampered = pco.clone();
        tampered.strategy_hash[0] = 0xFF;

        // Verification should fail
        assert!(tampered.verify_signature().is_err());
    }

    #[test]
    fn test_market_data_hash() {
        let market_data = MarketDataInput {
            timestamp: 1706745600,
            symbol: "BTCUSDT".to_string(),
            best_bid: 49990.0,
            best_ask: 50010.0,
            bid_size: 1.5,
            ask_size: 2.0,
            mid_price: 50000.0,
            volatility: Some(0.02),
            position: Some(0.5),
        };

        let hash = market_data.hash();

        // Hash should be deterministic
        let hash2 = market_data.hash();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_pco_serialization() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let order = create_test_order();

        let pco = ProofCarryingOrder::new(
            order,
            [1u8; 32],
            [2u8; 32],
            vec![[3u8; 32]],
            &signing_key,
        );

        // Serialize to JSON
        let json = pco.to_json().unwrap();

        // Deserialize back
        let pco2 = ProofCarryingOrder::from_json(&json).unwrap();

        // Verify the deserialized PCO
        assert!(pco2.verify_signature().unwrap());
        assert_eq!(pco.strategy_hash, pco2.strategy_hash);
    }
}
