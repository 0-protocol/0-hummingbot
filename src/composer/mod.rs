//! Graph Composition System
//!
//! Enables composing multiple 0-lang graphs into larger strategies.
//! Supports importing subgraphs, linking outputs to inputs, and
//! creating reusable strategy components.

mod linker;

pub use linker::{Connection, GraphLinker, LinkError};

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

/// Error types for graph composition
#[derive(Debug, thiserror::Error)]
pub enum ComposerError {
    #[error("Graph not found: {0}")]
    GraphNotFound(String),

    #[error("Cycle detected in composition")]
    CycleDetected,

    #[error("Invalid connection: {0}")]
    InvalidConnection(String),

    #[error("Output not found: {0} in graph {1}")]
    OutputNotFound(String, String),

    #[error("Input not found: {0} in graph {1}")]
    InputNotFound(String, String),

    #[error("Type mismatch: expected {0}, got {1}")]
    TypeMismatch(String, String),

    #[error("Link error: {0}")]
    LinkError(#[from] LinkError),
}

/// A node in the composed graph
#[derive(Debug, Clone)]
pub struct ComposedNode {
    /// Original node ID
    pub original_id: [u8; 32],

    /// Source graph hash
    pub source_graph: [u8; 32],

    /// Node name for reference
    pub name: String,

    /// Node type
    pub node_type: NodeType,

    /// Input connections (from other nodes)
    pub inputs: Vec<NodeInput>,
}

/// Types of nodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Constant,
    Operation,
    External,
    Branch,
    SubgraphInput,
    SubgraphOutput,
}

/// Input to a node
#[derive(Debug, Clone)]
pub struct NodeInput {
    /// Source node ID
    pub source_node: [u8; 32],

    /// Output index from source node
    pub output_index: usize,
}

/// Metadata about a subgraph
#[derive(Debug, Clone)]
pub struct SubgraphMetadata {
    /// Graph hash
    pub hash: [u8; 32],

    /// Graph name
    pub name: String,

    /// Input node names
    pub inputs: Vec<String>,

    /// Output node names
    pub outputs: Vec<String>,

    /// Description
    pub description: String,
}

/// Compose multiple graphs into one
pub struct GraphComposer {
    /// Imported graphs by hash
    graphs: HashMap<[u8; 32], SubgraphMetadata>,

    /// Composed nodes
    nodes: Vec<ComposedNode>,

    /// Connections between subgraphs
    connections: Vec<SubgraphConnection>,
}

/// Connection between subgraphs
#[derive(Debug, Clone)]
pub struct SubgraphConnection {
    /// Source graph hash
    pub from_graph: [u8; 32],

    /// Output name from source graph
    pub from_output: String,

    /// Target graph hash
    pub to_graph: [u8; 32],

    /// Input name in target graph
    pub to_input: String,
}

impl GraphComposer {
    /// Create a new graph composer
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    /// Import a subgraph
    pub fn import(&mut self, metadata: SubgraphMetadata) -> [u8; 32] {
        let hash = metadata.hash;
        self.graphs.insert(hash, metadata);
        hash
    }

    /// Add a connection between subgraphs
    pub fn connect(
        &mut self,
        from_graph: [u8; 32],
        from_output: &str,
        to_graph: [u8; 32],
        to_input: &str,
    ) -> Result<(), ComposerError> {
        // Verify graphs exist
        if !self.graphs.contains_key(&from_graph) {
            return Err(ComposerError::GraphNotFound(hex::encode(&from_graph[..8])));
        }
        if !self.graphs.contains_key(&to_graph) {
            return Err(ComposerError::GraphNotFound(hex::encode(&to_graph[..8])));
        }

        // Verify output exists
        let from_meta = self.graphs.get(&from_graph).unwrap();
        if !from_meta.outputs.contains(&from_output.to_string()) {
            return Err(ComposerError::OutputNotFound(
                from_output.to_string(),
                from_meta.name.clone(),
            ));
        }

        // Verify input exists
        let to_meta = self.graphs.get(&to_graph).unwrap();
        if !to_meta.inputs.contains(&to_input.to_string()) {
            return Err(ComposerError::InputNotFound(
                to_input.to_string(),
                to_meta.name.clone(),
            ));
        }

        self.connections.push(SubgraphConnection {
            from_graph,
            from_output: from_output.to_string(),
            to_graph,
            to_input: to_input.to_string(),
        });

        Ok(())
    }

    /// Check for cycles in the composition
    pub fn check_cycles(&self) -> Result<(), ComposerError> {
        // Build adjacency list
        let mut adj: HashMap<[u8; 32], Vec<[u8; 32]>> = HashMap::new();

        for conn in &self.connections {
            adj.entry(conn.from_graph)
                .or_default()
                .push(conn.to_graph);
        }

        // DFS for cycle detection
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for graph_hash in self.graphs.keys() {
            if self.has_cycle(*graph_hash, &adj, &mut visited, &mut rec_stack)? {
                return Err(ComposerError::CycleDetected);
            }
        }

        Ok(())
    }

    fn has_cycle(
        &self,
        node: [u8; 32],
        adj: &HashMap<[u8; 32], Vec<[u8; 32]>>,
        visited: &mut HashSet<[u8; 32]>,
        rec_stack: &mut HashSet<[u8; 32]>,
    ) -> Result<bool, ComposerError> {
        if rec_stack.contains(&node) {
            return Ok(true);
        }
        if visited.contains(&node) {
            return Ok(false);
        }

        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = adj.get(&node) {
            for neighbor in neighbors {
                if self.has_cycle(*neighbor, adj, visited, rec_stack)? {
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(&node);
        Ok(false)
    }

    /// Get topological order of graphs
    pub fn topological_order(&self) -> Result<Vec<[u8; 32]>, ComposerError> {
        self.check_cycles()?;

        // Build adjacency list and in-degree
        let mut adj: HashMap<[u8; 32], Vec<[u8; 32]>> = HashMap::new();
        let mut in_degree: HashMap<[u8; 32], usize> = HashMap::new();

        for graph_hash in self.graphs.keys() {
            in_degree.insert(*graph_hash, 0);
        }

        for conn in &self.connections {
            adj.entry(conn.from_graph)
                .or_default()
                .push(conn.to_graph);
            *in_degree.entry(conn.to_graph).or_insert(0) += 1;
        }

        // Kahn's algorithm
        let mut queue: Vec<[u8; 32]> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(hash, _)| *hash)
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node);

            if let Some(neighbors) = adj.get(&node) {
                for neighbor in neighbors {
                    let degree = in_degree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push(*neighbor);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Compose all graphs into a single composed graph
    pub fn compose(&self) -> Result<ComposedGraph, ComposerError> {
        self.check_cycles()?;

        let order = self.topological_order()?;

        // Build the composed graph
        let mut composed = ComposedGraph {
            name: "composed_graph".to_string(),
            subgraphs: order.iter().map(|h| self.graphs[h].clone()).collect(),
            connections: self.connections.clone(),
            hash: [0u8; 32],
        };

        // Calculate composed hash
        composed.hash = composed.calculate_hash();

        Ok(composed)
    }

    /// Get all imported graphs
    pub fn graphs(&self) -> &HashMap<[u8; 32], SubgraphMetadata> {
        &self.graphs
    }

    /// Get all connections
    pub fn connections(&self) -> &[SubgraphConnection] {
        &self.connections
    }
}

impl Default for GraphComposer {
    fn default() -> Self {
        Self::new()
    }
}

/// A composed graph from multiple subgraphs
#[derive(Debug, Clone)]
pub struct ComposedGraph {
    /// Name of the composed graph
    pub name: String,

    /// Subgraphs in topological order
    pub subgraphs: Vec<SubgraphMetadata>,

    /// Connections between subgraphs
    pub connections: Vec<SubgraphConnection>,

    /// Hash of the composed graph
    pub hash: [u8; 32],
}

impl ComposedGraph {
    /// Calculate hash of the composed graph
    fn calculate_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();

        hasher.update(self.name.as_bytes());

        for subgraph in &self.subgraphs {
            hasher.update(&subgraph.hash);
        }

        for conn in &self.connections {
            hasher.update(&conn.from_graph);
            hasher.update(conn.from_output.as_bytes());
            hasher.update(&conn.to_graph);
            hasher.update(conn.to_input.as_bytes());
        }

        hasher.finalize().into()
    }

    /// Get the execution order
    pub fn execution_order(&self) -> Vec<&SubgraphMetadata> {
        self.subgraphs.iter().collect()
    }

    /// Get connections for a specific subgraph
    pub fn connections_to(&self, graph_hash: &[u8; 32]) -> Vec<&SubgraphConnection> {
        self.connections
            .iter()
            .filter(|c| &c.to_graph == graph_hash)
            .collect()
    }

    /// Get connections from a specific subgraph
    pub fn connections_from(&self, graph_hash: &[u8; 32]) -> Vec<&SubgraphConnection> {
        self.connections
            .iter()
            .filter(|c| &c.from_graph == graph_hash)
            .collect()
    }
}

/// Builder for common strategy compositions
pub struct StrategyBuilder {
    composer: GraphComposer,
}

impl StrategyBuilder {
    pub fn new() -> Self {
        Self {
            composer: GraphComposer::new(),
        }
    }

    /// Add a market data source
    pub fn with_market_data(mut self, metadata: SubgraphMetadata) -> Self {
        self.composer.import(metadata);
        self
    }

    /// Add a risk check component
    pub fn with_risk_check(mut self, metadata: SubgraphMetadata) -> Self {
        self.composer.import(metadata);
        self
    }

    /// Add the main strategy
    pub fn with_strategy(mut self, metadata: SubgraphMetadata) -> Self {
        self.composer.import(metadata);
        self
    }

    /// Add an order executor
    pub fn with_executor(mut self, metadata: SubgraphMetadata) -> Self {
        self.composer.import(metadata);
        self
    }

    /// Connect components
    pub fn connect(
        mut self,
        from: [u8; 32],
        from_output: &str,
        to: [u8; 32],
        to_input: &str,
    ) -> Result<Self, ComposerError> {
        self.composer.connect(from, from_output, to, to_input)?;
        Ok(self)
    }

    /// Build the composed strategy
    pub fn build(self) -> Result<ComposedGraph, ComposerError> {
        self.composer.compose()
    }
}

impl Default for StrategyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_subgraph(name: &str) -> SubgraphMetadata {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());

        SubgraphMetadata {
            hash: hasher.finalize().into(),
            name: name.to_string(),
            inputs: vec!["input".to_string()],
            outputs: vec!["output".to_string()],
            description: format!("Test subgraph: {}", name),
        }
    }

    #[test]
    fn test_composer_basic() {
        let mut composer = GraphComposer::new();

        let market_data = create_test_subgraph("market_data");
        let strategy = create_test_subgraph("strategy");
        let executor = create_test_subgraph("executor");

        let md_hash = composer.import(market_data);
        let st_hash = composer.import(strategy);
        let ex_hash = composer.import(executor);

        // Connect: market_data -> strategy -> executor
        composer
            .connect(md_hash, "output", st_hash, "input")
            .unwrap();
        composer
            .connect(st_hash, "output", ex_hash, "input")
            .unwrap();

        let composed = composer.compose().unwrap();

        assert_eq!(composed.subgraphs.len(), 3);
        assert_eq!(composed.connections.len(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let mut composer = GraphComposer::new();

        let a = create_test_subgraph("a");
        let b = create_test_subgraph("b");

        let a_hash = composer.import(a);
        let b_hash = composer.import(b);

        // Create a cycle: a -> b -> a
        composer.connect(a_hash, "output", b_hash, "input").unwrap();
        composer.connect(b_hash, "output", a_hash, "input").unwrap();

        assert!(matches!(
            composer.check_cycles(),
            Err(ComposerError::CycleDetected)
        ));
    }

    #[test]
    fn test_topological_order() {
        let mut composer = GraphComposer::new();

        let a = create_test_subgraph("a");
        let b = create_test_subgraph("b");
        let c = create_test_subgraph("c");

        let a_hash = composer.import(a);
        let b_hash = composer.import(b);
        let c_hash = composer.import(c);

        // a -> b -> c
        composer.connect(a_hash, "output", b_hash, "input").unwrap();
        composer.connect(b_hash, "output", c_hash, "input").unwrap();

        let order = composer.topological_order().unwrap();

        // a should come before b, b before c
        let a_pos = order.iter().position(|h| h == &a_hash).unwrap();
        let b_pos = order.iter().position(|h| h == &b_hash).unwrap();
        let c_pos = order.iter().position(|h| h == &c_hash).unwrap();

        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_strategy_builder() {
        let market_data = create_test_subgraph("market_data");
        let risk_check = create_test_subgraph("risk_check");
        let strategy = create_test_subgraph("strategy");
        let executor = create_test_subgraph("executor");

        let md_hash = market_data.hash;
        let rc_hash = risk_check.hash;
        let st_hash = strategy.hash;
        let ex_hash = executor.hash;

        let composed = StrategyBuilder::new()
            .with_market_data(market_data)
            .with_risk_check(risk_check)
            .with_strategy(strategy)
            .with_executor(executor)
            .connect(md_hash, "output", st_hash, "input")
            .unwrap()
            .connect(st_hash, "output", rc_hash, "input")
            .unwrap()
            .connect(rc_hash, "output", ex_hash, "input")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(composed.subgraphs.len(), 4);
    }
}
