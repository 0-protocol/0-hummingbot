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
    pub fn load_strategy(&self, path: &Path) -> Result<RuntimeGraph, String> {
        RuntimeGraph::load_from_file(path).map_err(|e| format!("Failed to load graph: {:?}", e))
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

        let graph_path = Path::new(&self.config.strategy_path);
        let graph = self.load_strategy(graph_path)?;

        println!("✅ Strategy loaded successfully.");
        println!("⏳ Entering execution loop...");

        // Setup graceful shutdown channel
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        
        // Spawn a task to listen for ctrl-c
        tokio::spawn(async move {
            if let Ok(_) = tokio::signal::ctrl_c().await {
                let _ = shutdown_tx.send(true);
            }
        });

        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(self.config.interval_ms));

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    println!("🛑 Graceful shutdown initiated");
                    break;
                }
                _ = interval.tick() => {
                    // Execute the graph for this iteration
                    match self.execute_once(&graph) {
                        Ok(tensors) => {
                            if !tensors.is_empty() {
                                let decision = &tensors[0];
                                println!("⚡ Decision emitted: {:?}", decision);
                            } else {
                                println!("⚡ Graph executed successfully, no output tensor.");
                            }
                        }
                        Err(e) => {
                            println!("❌ Execution error: {}", e);
                        }
                    }
                }
            }
        }

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
