//! PCO Builder
//!
//! Builds Proof-Carrying Orders during strategy execution,
//! recording the execution trace as nodes are evaluated.

use super::{MarketDataInput, Order, PcoError, ProofCarryingOrder};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

/// Builder for creating Proof-Carrying Orders
///
/// Usage:
/// ```ignore
/// let mut builder = PcoBuilder::new(signing_key);
///
/// // As strategy executes, record each node
/// builder.record_node("get_mid_price", &node_hash);
/// builder.record_node("calc_spread", &node_hash);
/// // ...
///
/// // When ready to place order
/// let pco = builder.build(order, strategy_hash, market_data)?;
/// ```
pub struct PcoBuilder {
    signing_key: SigningKey,
    execution_trace: Vec<[u8; 32]>,
    node_names: Vec<String>,
}

impl PcoBuilder {
    /// Create a new PCO builder with the signing key
    pub fn new(signing_key: SigningKey) -> Self {
        Self {
            signing_key,
            execution_trace: Vec::new(),
            node_names: Vec::new(),
        }
    }

    /// Record a node evaluation in the execution trace
    ///
    /// Call this for each node as it's evaluated during strategy execution.
    pub fn record_node(&mut self, node_name: &str, node_hash: &[u8; 32]) {
        self.execution_trace.push(*node_hash);
        self.node_names.push(node_name.to_string());
    }

    /// Record a node by computing its hash from name
    pub fn record_node_by_name(&mut self, node_name: &str) {
        let hash = Self::hash_node_name(node_name);
        self.record_node(node_name, &hash);
    }

    /// Get the number of recorded nodes
    pub fn trace_length(&self) -> usize {
        self.execution_trace.len()
    }

    /// Get the recorded node names (for debugging)
    pub fn node_names(&self) -> &[String] {
        &self.node_names
    }

    /// Clear the execution trace (for reuse)
    pub fn clear(&mut self) {
        self.execution_trace.clear();
        self.node_names.clear();
    }

    /// Build the final Proof-Carrying Order
    pub fn build(
        self,
        order: Order,
        strategy_hash: [u8; 32],
        market_data: &MarketDataInput,
    ) -> Result<ProofCarryingOrder, PcoError> {
        if self.execution_trace.is_empty() {
            return Err(PcoError::InvalidExecutionTrace(
                "No nodes recorded in execution trace".into(),
            ));
        }

        let input_hash = market_data.hash();

        Ok(ProofCarryingOrder::new(
            order,
            strategy_hash,
            input_hash,
            self.execution_trace,
            &self.signing_key,
        ))
    }

    /// Build with explicit input hash
    pub fn build_with_input_hash(
        self,
        order: Order,
        strategy_hash: [u8; 32],
        input_hash: [u8; 32],
    ) -> Result<ProofCarryingOrder, PcoError> {
        if self.execution_trace.is_empty() {
            return Err(PcoError::InvalidExecutionTrace(
                "No nodes recorded in execution trace".into(),
            ));
        }

        Ok(ProofCarryingOrder::new(
            order,
            strategy_hash,
            input_hash,
            self.execution_trace,
            &self.signing_key,
        ))
    }

    /// Hash a node name to get its ID
    pub fn hash_node_name(name: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.finalize().into()
    }

    /// Get the execution trace for inspection
    pub fn execution_trace(&self) -> &[[u8; 32]] {
        &self.execution_trace
    }
}

/// Convenient macro for building PCOs
///
/// Usage:
/// ```ignore
/// let pco = build_pco!(
///     signing_key,
///     order,
///     strategy_hash,
///     market_data,
///     nodes: ["get_mid_price", "calc_spread", "place_order"]
/// );
/// ```
#[macro_export]
macro_rules! build_pco {
    ($signing_key:expr, $order:expr, $strategy_hash:expr, $market_data:expr, nodes: [$($node:expr),*]) => {{
        let mut builder = PcoBuilder::new($signing_key);
        $(
            builder.record_node_by_name($node);
        )*
        builder.build($order, $strategy_hash, $market_data)
    }};
}

/// Builder for batch PCO creation
///
/// Useful when a single strategy execution generates multiple orders
pub struct BatchPcoBuilder {
    signing_key: SigningKey,
    strategy_hash: [u8; 32],
    base_trace: Vec<[u8; 32]>,
}

impl BatchPcoBuilder {
    /// Create a new batch builder
    pub fn new(signing_key: SigningKey, strategy_hash: [u8; 32]) -> Self {
        Self {
            signing_key,
            strategy_hash,
            base_trace: Vec::new(),
        }
    }

    /// Record shared execution trace (common to all orders)
    pub fn record_shared_node(&mut self, node_name: &str) {
        let hash = PcoBuilder::hash_node_name(node_name);
        self.base_trace.push(hash);
    }

    /// Build a PCO for a specific order with additional trace nodes
    pub fn build_for_order(
        &self,
        order: Order,
        market_data: &MarketDataInput,
        additional_nodes: &[&str],
    ) -> Result<ProofCarryingOrder, PcoError> {
        let mut trace = self.base_trace.clone();
        for node in additional_nodes {
            trace.push(PcoBuilder::hash_node_name(node));
        }

        if trace.is_empty() {
            return Err(PcoError::InvalidExecutionTrace(
                "No nodes in execution trace".into(),
            ));
        }

        let input_hash = market_data.hash();

        Ok(ProofCarryingOrder::new(
            order,
            self.strategy_hash,
            input_hash,
            trace,
            &self.signing_key,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pco::{OrderSide, OrderType};
    use rand::rngs::OsRng;

    fn create_test_signing_key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

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

    fn create_test_market_data() -> MarketDataInput {
        MarketDataInput {
            timestamp: 1706745600,
            symbol: "BTCUSDT".to_string(),
            best_bid: 49990.0,
            best_ask: 50010.0,
            bid_size: 1.5,
            ask_size: 2.0,
            mid_price: 50000.0,
            volatility: Some(0.02),
            position: Some(0.0),
        }
    }

    #[test]
    fn test_pco_builder() {
        let signing_key = create_test_signing_key();
        let mut builder = PcoBuilder::new(signing_key);

        // Record some nodes
        builder.record_node_by_name("get_mid_price");
        builder.record_node_by_name("calc_spread");
        builder.record_node_by_name("place_bid");

        assert_eq!(builder.trace_length(), 3);

        let order = create_test_order();
        let market_data = create_test_market_data();
        let strategy_hash = [0u8; 32];

        let pco = builder.build(order, strategy_hash, &market_data).unwrap();

        // Verify the PCO
        assert!(pco.verify_signature().unwrap());
        assert_eq!(pco.execution_trace.len(), 3);
    }

    #[test]
    fn test_empty_trace_fails() {
        let signing_key = create_test_signing_key();
        let builder = PcoBuilder::new(signing_key);

        let order = create_test_order();
        let market_data = create_test_market_data();
        let strategy_hash = [0u8; 32];

        let result = builder.build(order, strategy_hash, &market_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_builder() {
        let signing_key = create_test_signing_key();
        let strategy_hash = [1u8; 32];
        let mut batch = BatchPcoBuilder::new(signing_key, strategy_hash);

        // Record shared trace
        batch.record_shared_node("get_mid_price");
        batch.record_shared_node("calc_spread");

        let market_data = create_test_market_data();

        // Build bid order PCO
        let bid_order = Order {
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity: 0.01,
            price: Some(49990.0),
            stop_price: None,
            time_in_force: "GTC".to_string(),
            client_order_id: "bid-001".to_string(),
        };
        let bid_pco = batch
            .build_for_order(bid_order, &market_data, &["place_bid"])
            .unwrap();

        // Build ask order PCO
        let ask_order = Order {
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Limit,
            quantity: 0.01,
            price: Some(50010.0),
            stop_price: None,
            time_in_force: "GTC".to_string(),
            client_order_id: "ask-001".to_string(),
        };
        let ask_pco = batch
            .build_for_order(ask_order, &market_data, &["place_ask"])
            .unwrap();

        // Both should verify
        assert!(bid_pco.verify_signature().unwrap());
        assert!(ask_pco.verify_signature().unwrap());

        // They share the same base trace but have different final nodes
        assert_eq!(bid_pco.execution_trace.len(), 3);
        assert_eq!(ask_pco.execution_trace.len(), 3);
    }

    #[test]
    fn test_node_hash_determinism() {
        let hash1 = PcoBuilder::hash_node_name("get_mid_price");
        let hash2 = PcoBuilder::hash_node_name("get_mid_price");
        assert_eq!(hash1, hash2);

        let hash3 = PcoBuilder::hash_node_name("different_node");
        assert_ne!(hash1, hash3);
    }
}
