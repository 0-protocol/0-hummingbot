//! Runtime for executing 0-hummingbot strategies
//!
//! Handles the execution loop, market data, and order management.

use std::path::Path;
use std::sync::Arc;
use zerolang::{ExternalResolver, RuntimeGraph, Tensor, VM};

use crate::resolvers::HttpResolver;

/// Trading runtime configuration
pub struct RuntimeConfig {
    /// Strategy graph path
    pub strategy_path: String,
    /// Connector name
    pub connector: String,
    /// Trading pair
    pub pair: String,
    /// Execution interval in milliseconds
    pub interval_ms: u64,
    /// Paper trading mode
    pub paper_mode: bool,
}

/// The trading runtime
pub struct TradingRuntime {
    config: RuntimeConfig,
    vm: VM,
    http_resolver: Arc<HttpResolver>,
}

impl TradingRuntime {
    /// Create a new trading runtime
    pub fn new(config: RuntimeConfig) -> Self {
        let http_resolver = Arc::new(HttpResolver::new());
        let vm = VM::new().with_external_resolver(http_resolver.clone() as Arc<dyn ExternalResolver>);

        Self {
            config,
            vm,
            http_resolver,
        }
    }

    /// Load a strategy graph from file
    pub fn load_strategy(&self, _path: &Path) -> Result<RuntimeGraph, String> {
        // TODO: Implement graph loading from .0 files
        Err("Graph loading not yet implemented".to_string())
    }

    /// Execute a single iteration of the strategy
    pub fn execute_once(&mut self, graph: &RuntimeGraph) -> Result<Vec<Tensor>, String> {
        self.vm
            .execute(graph)
            .map_err(|e| format!("Execution error: {}", e))
    }

    /// Run the strategy continuously
    pub async fn run(&mut self) -> Result<(), String> {
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│  TRADING RUNTIME                                            │");
        println!("├─────────────────────────────────────────────────────────────┤");
        println!("│  Strategy: {}", self.config.strategy_path);
        println!("│  Connector: {}", self.config.connector);
        println!("│  Pair: {}", self.config.pair);
        println!("│  Interval: {}ms", self.config.interval_ms);
        println!("│  Mode: {}", if self.config.paper_mode { "Paper" } else { "Live" });
        println!("├─────────────────────────────────────────────────────────────┤");
        println!("│  Status: Runtime loop not yet implemented                   │");
        println!("└─────────────────────────────────────────────────────────────┘");

        // TODO: Implement the execution loop
        // 1. Load strategy graph
        // 2. Fetch market data
        // 3. Execute graph
        // 4. Process decision tensor
        // 5. Place orders if confidence > threshold
        // 6. Sleep for interval
        // 7. Repeat

        Ok(())
    }
}

/// Order decision from strategy execution
#[derive(Debug)]
pub struct OrderDecision {
    /// Whether to place an order
    pub should_order: bool,
    /// Order side (buy/sell)
    pub side: OrderSide,
    /// Order quantity
    pub quantity: f32,
    /// Order price (for limit orders)
    pub price: Option<f32>,
    /// Confidence level
    pub confidence: f32,
}

/// Order side
#[derive(Debug, Clone, Copy)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderDecision {
    /// Create from strategy output tensor
    pub fn from_tensor(tensor: &Tensor) -> Option<Self> {
        // Expected tensor format:
        // [should_order, side, quantity, price] with confidence
        
        if tensor.data.len() < 4 {
            return None;
        }

        let should_order = tensor.data[0] > 0.5;
        let side = if tensor.data[1] > 0.5 {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let quantity = tensor.data[2];
        let price = if tensor.data[3] > 0.0 {
            Some(tensor.data[3])
        } else {
            None
        };

        Some(Self {
            should_order,
            side,
            quantity,
            price,
            confidence: tensor.confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_decision_from_tensor() {
        let tensor = Tensor {
            shape: vec![4],
            data: vec![1.0, 1.0, 0.5, 100.0], // should_order=true, buy, 0.5 qty, $100
            confidence: 0.9,
        };

        let decision = OrderDecision::from_tensor(&tensor).unwrap();
        
        assert!(decision.should_order);
        assert!(matches!(decision.side, OrderSide::Buy));
        assert_eq!(decision.quantity, 0.5);
        assert_eq!(decision.price, Some(100.0));
        assert_eq!(decision.confidence, 0.9);
    }
}
