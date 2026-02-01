//! Graph Linker
//!
//! Handles the low-level linking of graph outputs to inputs,
//! including type checking and connection validation.

use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Error types for linking operations
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Port not found: {0} on node {1}")]
    PortNotFound(String, String),

    #[error("Type mismatch: expected shape {0:?}, got {1:?}")]
    ShapeMismatch(Vec<usize>, Vec<usize>),

    #[error("Already connected: {0}")]
    AlreadyConnected(String),

    #[error("Invalid graph: {0}")]
    InvalidGraph(String),
}

/// A connection between graph nodes
#[derive(Debug, Clone)]
pub struct Connection {
    /// Source node ID
    pub from_node: [u8; 32],

    /// Output port index on source node
    pub from_port: usize,

    /// Target node ID
    pub to_node: [u8; 32],

    /// Input port index on target node
    pub to_port: usize,
}

impl Connection {
    /// Create a new connection
    pub fn new(from_node: [u8; 32], from_port: usize, to_node: [u8; 32], to_port: usize) -> Self {
        Self {
            from_node,
            from_port,
            to_node,
            to_port,
        }
    }

    /// Create connection from node names
    pub fn from_names(from_name: &str, from_port: usize, to_name: &str, to_port: usize) -> Self {
        Self {
            from_node: Self::hash_name(from_name),
            from_port,
            to_node: Self::hash_name(to_name),
            to_port,
        }
    }

    fn hash_name(name: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.finalize().into()
    }
}

/// Port information for a node
#[derive(Debug, Clone)]
pub struct PortInfo {
    /// Port name
    pub name: String,

    /// Expected tensor shape (empty for any shape)
    pub shape: Vec<usize>,

    /// Whether this port is required
    pub required: bool,
}

/// Node information for linking
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node ID
    pub id: [u8; 32],

    /// Node name
    pub name: String,

    /// Input ports
    pub inputs: Vec<PortInfo>,

    /// Output ports
    pub outputs: Vec<PortInfo>,
}

/// Graph linker for connecting nodes
pub struct GraphLinker {
    /// Node information by ID
    nodes: HashMap<[u8; 32], NodeInfo>,

    /// Established connections
    connections: Vec<Connection>,

    /// Track which input ports are connected
    connected_inputs: HashMap<([u8; 32], usize), [u8; 32]>,
}

impl GraphLinker {
    /// Create a new graph linker
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
            connected_inputs: HashMap::new(),
        }
    }

    /// Register a node for linking
    pub fn register_node(&mut self, node: NodeInfo) {
        self.nodes.insert(node.id, node);
    }

    /// Register a node by name
    pub fn register_node_by_name(
        &mut self,
        name: &str,
        inputs: Vec<PortInfo>,
        outputs: Vec<PortInfo>,
    ) {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());

        let node = NodeInfo {
            id: hasher.finalize().into(),
            name: name.to_string(),
            inputs,
            outputs,
        };

        self.register_node(node);
    }

    /// Add a connection between nodes
    pub fn connect(&mut self, connection: Connection) -> Result<(), LinkError> {
        // Verify source node exists
        let from_node = self
            .nodes
            .get(&connection.from_node)
            .ok_or_else(|| LinkError::NodeNotFound(hex::encode(&connection.from_node[..8])))?;

        // Verify target node exists
        let to_node = self
            .nodes
            .get(&connection.to_node)
            .ok_or_else(|| LinkError::NodeNotFound(hex::encode(&connection.to_node[..8])))?;

        // Verify output port exists
        if connection.from_port >= from_node.outputs.len() {
            return Err(LinkError::PortNotFound(
                format!("output[{}]", connection.from_port),
                from_node.name.clone(),
            ));
        }

        // Verify input port exists
        if connection.to_port >= to_node.inputs.len() {
            return Err(LinkError::PortNotFound(
                format!("input[{}]", connection.to_port),
                to_node.name.clone(),
            ));
        }

        // Check if input is already connected
        let input_key = (connection.to_node, connection.to_port);
        if self.connected_inputs.contains_key(&input_key) {
            return Err(LinkError::AlreadyConnected(format!(
                "{}:input[{}]",
                to_node.name, connection.to_port
            )));
        }

        // Type check: verify shapes are compatible
        let from_shape = &from_node.outputs[connection.from_port].shape;
        let to_shape = &to_node.inputs[connection.to_port].shape;

        if !from_shape.is_empty() && !to_shape.is_empty() && from_shape != to_shape {
            return Err(LinkError::ShapeMismatch(to_shape.clone(), from_shape.clone()));
        }

        // Add connection
        self.connected_inputs
            .insert(input_key, connection.from_node);
        self.connections.push(connection);

        Ok(())
    }

    /// Connect nodes by name
    pub fn connect_by_name(
        &mut self,
        from_name: &str,
        from_port: usize,
        to_name: &str,
        to_port: usize,
    ) -> Result<(), LinkError> {
        let connection = Connection::from_names(from_name, from_port, to_name, to_port);
        self.connect(connection)
    }

    /// Validate that all required inputs are connected
    pub fn validate(&self) -> Result<(), LinkError> {
        for node in self.nodes.values() {
            for (port_idx, port) in node.inputs.iter().enumerate() {
                if port.required && !self.connected_inputs.contains_key(&(node.id, port_idx)) {
                    return Err(LinkError::InvalidGraph(format!(
                        "Required input {}:{} is not connected",
                        node.name, port.name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Get all connections
    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    /// Get connections to a specific node
    pub fn connections_to(&self, node_id: &[u8; 32]) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| &c.to_node == node_id)
            .collect()
    }

    /// Get connections from a specific node
    pub fn connections_from(&self, node_id: &[u8; 32]) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| &c.from_node == node_id)
            .collect()
    }

    /// Get the source of an input port
    pub fn source_of(&self, node_id: &[u8; 32], port: usize) -> Option<([u8; 32], usize)> {
        self.connections
            .iter()
            .find(|c| &c.to_node == node_id && c.to_port == port)
            .map(|c| (c.from_node, c.from_port))
    }

    /// Get all nodes that depend on a given node
    pub fn dependents(&self, node_id: &[u8; 32]) -> Vec<[u8; 32]> {
        self.connections
            .iter()
            .filter(|c| &c.from_node == node_id)
            .map(|c| c.to_node)
            .collect()
    }

    /// Get all nodes that a given node depends on
    pub fn dependencies(&self, node_id: &[u8; 32]) -> Vec<[u8; 32]> {
        self.connections
            .iter()
            .filter(|c| &c.to_node == node_id)
            .map(|c| c.from_node)
            .collect()
    }
}

impl Default for GraphLinker {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for building common node patterns
pub struct NodeBuilder {
    name: String,
    inputs: Vec<PortInfo>,
    outputs: Vec<PortInfo>,
}

impl NodeBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Add an input port
    pub fn with_input(mut self, name: &str, shape: Vec<usize>, required: bool) -> Self {
        self.inputs.push(PortInfo {
            name: name.to_string(),
            shape,
            required,
        });
        self
    }

    /// Add a scalar input
    pub fn with_scalar_input(self, name: &str, required: bool) -> Self {
        self.with_input(name, vec![1], required)
    }

    /// Add an output port
    pub fn with_output(mut self, name: &str, shape: Vec<usize>) -> Self {
        self.outputs.push(PortInfo {
            name: name.to_string(),
            shape,
            required: false, // outputs are never "required"
        });
        self
    }

    /// Add a scalar output
    pub fn with_scalar_output(self, name: &str) -> Self {
        self.with_output(name, vec![1])
    }

    /// Build the node info
    pub fn build(self) -> NodeInfo {
        let mut hasher = Sha256::new();
        hasher.update(self.name.as_bytes());

        NodeInfo {
            id: hasher.finalize().into(),
            name: self.name,
            inputs: self.inputs,
            outputs: self.outputs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(name: &str, num_inputs: usize, num_outputs: usize) -> NodeInfo {
        let mut builder = NodeBuilder::new(name);

        for i in 0..num_inputs {
            builder = builder.with_scalar_input(&format!("in{}", i), true);
        }

        for i in 0..num_outputs {
            builder = builder.with_scalar_output(&format!("out{}", i));
        }

        builder.build()
    }

    #[test]
    fn test_basic_linking() {
        let mut linker = GraphLinker::new();

        let node_a = create_test_node("a", 0, 1);
        let node_b = create_test_node("b", 1, 1);
        let node_c = create_test_node("c", 1, 0);

        linker.register_node(node_a.clone());
        linker.register_node(node_b.clone());
        linker.register_node(node_c.clone());

        // a -> b -> c
        linker.connect_by_name("a", 0, "b", 0).unwrap();
        linker.connect_by_name("b", 0, "c", 0).unwrap();

        assert_eq!(linker.connections().len(), 2);
    }

    #[test]
    fn test_double_connection_fails() {
        let mut linker = GraphLinker::new();

        let node_a = create_test_node("a", 0, 1);
        let node_b = create_test_node("b", 0, 1);
        let node_c = create_test_node("c", 1, 0);

        linker.register_node(node_a);
        linker.register_node(node_b);
        linker.register_node(node_c);

        linker.connect_by_name("a", 0, "c", 0).unwrap();

        // Try to connect another node to the same input
        let result = linker.connect_by_name("b", 0, "c", 0);
        assert!(matches!(result, Err(LinkError::AlreadyConnected(_))));
    }

    #[test]
    fn test_validation() {
        let mut linker = GraphLinker::new();

        let node_a = create_test_node("a", 0, 1);
        let node_b = create_test_node("b", 1, 0);

        linker.register_node(node_a);
        linker.register_node(node_b);

        // Don't connect - validation should fail
        assert!(linker.validate().is_err());

        // Connect and try again
        linker.connect_by_name("a", 0, "b", 0).unwrap();
        assert!(linker.validate().is_ok());
    }

    #[test]
    fn test_dependencies() {
        let mut linker = GraphLinker::new();

        let node_a = create_test_node("a", 0, 1);
        let node_b = create_test_node("b", 1, 1);
        let node_c = create_test_node("c", 1, 0);

        let a_id = node_a.id;
        let b_id = node_b.id;
        let c_id = node_c.id;

        linker.register_node(node_a);
        linker.register_node(node_b);
        linker.register_node(node_c);

        linker.connect_by_name("a", 0, "b", 0).unwrap();
        linker.connect_by_name("b", 0, "c", 0).unwrap();

        // b depends on a
        let deps = linker.dependencies(&b_id);
        assert!(deps.contains(&a_id));

        // a has b as dependent
        let dependents = linker.dependents(&a_id);
        assert!(dependents.contains(&b_id));
    }

    #[test]
    fn test_node_builder() {
        let node = NodeBuilder::new("test_node")
            .with_scalar_input("price", true)
            .with_input("orderbook", vec![5, 2], true)
            .with_scalar_output("decision")
            .with_output("orders", vec![10])
            .build();

        assert_eq!(node.name, "test_node");
        assert_eq!(node.inputs.len(), 2);
        assert_eq!(node.outputs.len(), 2);
        assert_eq!(node.inputs[0].name, "price");
        assert_eq!(node.inputs[1].shape, vec![5, 2]);
    }
}
