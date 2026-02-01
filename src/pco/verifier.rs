//! PCO Verifier
//!
//! Verifies Proof-Carrying Orders against strategy graphs,
//! ensuring orders were generated according to the claimed strategy.

use super::{PcoError, ProofCarryingOrder};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// Verifier for Proof-Carrying Orders
///
/// Provides comprehensive verification of PCOs including:
/// - Signature verification
/// - Execution trace validation
/// - Strategy hash verification
/// - Timestamp validation
pub struct PcoVerifier {
    /// Known valid strategy hashes
    known_strategies: HashSet<[u8; 32]>,

    /// Known valid agent public keys
    known_agents: HashSet<[u8; 32]>,

    /// Maximum age for valid PCOs (seconds)
    max_age_secs: u64,
}

impl PcoVerifier {
    /// Create a new verifier with default settings
    pub fn new() -> Self {
        Self {
            known_strategies: HashSet::new(),
            known_agents: HashSet::new(),
            max_age_secs: 3600, // 1 hour default
        }
    }

    /// Set maximum age for valid PCOs
    pub fn with_max_age(mut self, max_age_secs: u64) -> Self {
        self.max_age_secs = max_age_secs;
        self
    }

    /// Register a known valid strategy hash
    pub fn register_strategy(&mut self, strategy_hash: [u8; 32]) {
        self.known_strategies.insert(strategy_hash);
    }

    /// Register a known valid agent public key
    pub fn register_agent(&mut self, agent_pubkey: [u8; 32]) {
        self.known_agents.insert(agent_pubkey);
    }

    /// Perform full verification of a PCO
    pub fn verify(&self, pco: &ProofCarryingOrder) -> Result<VerificationResult, PcoError> {
        let mut result = VerificationResult::new();

        // 1. Verify signature
        match pco.verify_signature() {
            Ok(true) => result.signature_valid = true,
            Ok(false) => result.signature_valid = false,
            Err(e) => {
                result.signature_valid = false;
                result.errors.push(format!("Signature error: {}", e));
            }
        }

        // 2. Verify timestamp
        match pco.verify_timestamp(self.max_age_secs) {
            Ok(true) => result.timestamp_valid = true,
            Ok(false) => result.timestamp_valid = false,
            Err(e) => {
                result.timestamp_valid = false;
                result.errors.push(format!("Timestamp error: {}", e));
            }
        }

        // 3. Check if strategy is known
        result.strategy_known = self.known_strategies.contains(&pco.strategy_hash);
        if !result.strategy_known && !self.known_strategies.is_empty() {
            result
                .warnings
                .push("Strategy hash not in known strategies".into());
        }

        // 4. Check if agent is known
        result.agent_known = self.known_agents.contains(&pco.agent_pubkey);
        if !result.agent_known && !self.known_agents.is_empty() {
            result
                .warnings
                .push("Agent public key not in known agents".into());
        }

        // 5. Verify execution trace is non-empty
        if pco.execution_trace.is_empty() {
            result.trace_valid = false;
            result.errors.push("Execution trace is empty".into());
        } else {
            result.trace_valid = true;
            result.trace_length = pco.execution_trace.len();
        }

        // Calculate overall validity
        result.is_valid = result.signature_valid && result.timestamp_valid && result.trace_valid;

        Ok(result)
    }

    /// Verify that an execution trace matches a valid path through a strategy
    ///
    /// This requires the actual strategy graph to validate against.
    pub fn verify_execution_trace(
        &self,
        pco: &ProofCarryingOrder,
        strategy_node_hashes: &[[u8; 32]],
        valid_paths: &[Vec<usize>],
    ) -> Result<bool, PcoError> {
        // Create a set of valid node hashes
        let valid_nodes: HashSet<[u8; 32]> = strategy_node_hashes.iter().cloned().collect();

        // Check that all trace nodes are valid strategy nodes
        for trace_node in &pco.execution_trace {
            if !valid_nodes.contains(trace_node) {
                return Err(PcoError::InvalidExecutionTrace(format!(
                    "Node {} not found in strategy",
                    hex::encode(&trace_node[..8])
                )));
            }
        }

        // Check if the trace matches any valid path
        if valid_paths.is_empty() {
            // No path constraints, just check nodes are valid
            return Ok(true);
        }

        // Convert trace to node indices
        let trace_indices: Vec<usize> = pco
            .execution_trace
            .iter()
            .filter_map(|hash| {
                strategy_node_hashes
                    .iter()
                    .position(|node| node == hash)
            })
            .collect();

        // Check if trace matches any valid path
        for valid_path in valid_paths {
            if trace_indices == *valid_path {
                return Ok(true);
            }
        }

        Err(PcoError::InvalidExecutionTrace(
            "Execution trace does not match any valid strategy path".into(),
        ))
    }

    /// Batch verify multiple PCOs
    pub fn verify_batch(
        &self,
        pcos: &[ProofCarryingOrder],
    ) -> Vec<Result<VerificationResult, PcoError>> {
        pcos.iter().map(|pco| self.verify(pco)).collect()
    }

    /// Verify PCOs are from the same strategy execution
    ///
    /// Useful for verifying related orders (e.g., bid and ask from same MM cycle)
    pub fn verify_same_execution(&self, pcos: &[ProofCarryingOrder]) -> Result<bool, PcoError> {
        if pcos.len() < 2 {
            return Ok(true);
        }

        let reference = &pcos[0];

        for pco in &pcos[1..] {
            // Must have same strategy
            if pco.strategy_hash != reference.strategy_hash {
                return Err(PcoError::StrategyHashMismatch);
            }

            // Must have same input
            if pco.input_hash != reference.input_hash {
                return Err(PcoError::InputHashMismatch);
            }

            // Must have same agent
            if pco.agent_pubkey != reference.agent_pubkey {
                return Err(PcoError::InvalidPublicKey(
                    "Different agents for same execution".into(),
                ));
            }

            // Timestamps should be close (within 1 second)
            let time_diff = if pco.timestamp > reference.timestamp {
                pco.timestamp - reference.timestamp
            } else {
                reference.timestamp - pco.timestamp
            };

            if time_diff > 1 {
                return Err(PcoError::TimestampError(
                    "Timestamps differ by more than 1 second".into(),
                ));
            }
        }

        Ok(true)
    }
}

impl Default for PcoVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of PCO verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Overall validity
    pub is_valid: bool,

    /// Signature verification passed
    pub signature_valid: bool,

    /// Timestamp is within acceptable range
    pub timestamp_valid: bool,

    /// Strategy hash is known
    pub strategy_known: bool,

    /// Agent public key is known
    pub agent_known: bool,

    /// Execution trace is valid
    pub trace_valid: bool,

    /// Number of nodes in execution trace
    pub trace_length: usize,

    /// Error messages
    pub errors: Vec<String>,

    /// Warning messages
    pub warnings: Vec<String>,
}

impl VerificationResult {
    fn new() -> Self {
        Self {
            is_valid: false,
            signature_valid: false,
            timestamp_valid: false,
            strategy_known: false,
            agent_known: false,
            trace_valid: false,
            trace_length: 0,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "PCO Verification: valid={}, sig={}, time={}, trace_len={}",
            self.is_valid, self.signature_valid, self.timestamp_valid, self.trace_length
        )
    }
}

/// Audit log entry for PCO verification
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// PCO hash
    pub pco_hash: [u8; 32],

    /// Strategy hash
    pub strategy_hash: [u8; 32],

    /// Agent public key
    pub agent_pubkey: [u8; 32],

    /// Timestamp of verification
    pub verified_at: u64,

    /// Verification result
    pub result: VerificationResult,
}

/// Audit log for tracking PCO verifications
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Record a verification
    pub fn record(&mut self, pco: &ProofCarryingOrder, result: VerificationResult) {
        let entry = AuditEntry {
            pco_hash: pco.hash(),
            strategy_hash: pco.strategy_hash,
            agent_pubkey: pco.agent_pubkey,
            verified_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            result,
        };
        self.entries.push(entry);
    }

    /// Get all entries
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get entries for a specific strategy
    pub fn entries_for_strategy(&self, strategy_hash: &[u8; 32]) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| &e.strategy_hash == strategy_hash)
            .collect()
    }

    /// Get entries for a specific agent
    pub fn entries_for_agent(&self, agent_pubkey: &[u8; 32]) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| &e.agent_pubkey == agent_pubkey)
            .collect()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pco::{Order, OrderSide, OrderType};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn create_test_pco() -> (ProofCarryingOrder, SigningKey) {
        let signing_key = SigningKey::generate(&mut OsRng);

        let order = Order {
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity: 0.01,
            price: Some(50000.0),
            stop_price: None,
            time_in_force: "GTC".to_string(),
            client_order_id: "test-001".to_string(),
        };

        let pco = ProofCarryingOrder::new(
            order,
            [1u8; 32],
            [2u8; 32],
            vec![[3u8; 32], [4u8; 32]],
            &signing_key,
        );

        (pco, signing_key)
    }

    #[test]
    fn test_verifier_basic() {
        let verifier = PcoVerifier::new();
        let (pco, _) = create_test_pco();

        let result = verifier.verify(&pco).unwrap();

        assert!(result.is_valid);
        assert!(result.signature_valid);
        assert!(result.timestamp_valid);
        assert!(result.trace_valid);
        assert_eq!(result.trace_length, 2);
    }

    #[test]
    fn test_verifier_with_known_strategy() {
        let mut verifier = PcoVerifier::new();
        let (pco, _) = create_test_pco();

        // Register the strategy
        verifier.register_strategy(pco.strategy_hash);

        let result = verifier.verify(&pco).unwrap();

        assert!(result.is_valid);
        assert!(result.strategy_known);
    }

    #[test]
    fn test_verifier_with_known_agent() {
        let mut verifier = PcoVerifier::new();
        let (pco, _) = create_test_pco();

        // Register the agent
        verifier.register_agent(pco.agent_pubkey);

        let result = verifier.verify(&pco).unwrap();

        assert!(result.is_valid);
        assert!(result.agent_known);
    }

    #[test]
    fn test_audit_log() {
        let verifier = PcoVerifier::new();
        let mut audit_log = AuditLog::new();
        let (pco, _) = create_test_pco();

        let result = verifier.verify(&pco).unwrap();
        audit_log.record(&pco, result);

        assert_eq!(audit_log.entries().len(), 1);
    }

    #[test]
    fn test_same_execution_verification() {
        let verifier = PcoVerifier::new();
        let signing_key = SigningKey::generate(&mut OsRng);

        let strategy_hash = [1u8; 32];
        let input_hash = [2u8; 32];

        // Create two orders from same execution
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

        let pco1 = ProofCarryingOrder::new(
            bid_order,
            strategy_hash,
            input_hash,
            vec![[3u8; 32]],
            &signing_key,
        );

        let pco2 = ProofCarryingOrder::new(
            ask_order,
            strategy_hash,
            input_hash,
            vec![[4u8; 32]],
            &signing_key,
        );

        // They should be from same execution
        assert!(verifier.verify_same_execution(&[pco1, pco2]).unwrap());
    }
}
