//! Consensus types and data structures

use crate::etl::Block;
use serde::{Deserialize, Serialize};

/// Consensus result
/// 
/// Note: This is used in the ConsensusAlgorithm trait and tests.
/// Currently, main.rs uses PBFT directly, but this type is reserved for
/// future use with the trait-based approach.
#[allow(dead_code)] // Used in trait definitions and tests
#[derive(Debug, Clone)]
pub enum ConsensusResult {
    /// Consensus reached
    Committed(Block),
    /// Consensus pending (for eventual consistency algorithms)
    Pending,
    /// Consensus failed
    Rejected(String),
}

/// Consensus requirements
/// 
/// Note: This is used in the ConsensusAlgorithm trait and tests.
/// Currently, main.rs uses PBFT directly, but this type is reserved for
/// future use with the trait-based approach.
#[allow(dead_code)] // Used in trait definitions and tests
#[derive(Debug, Clone)]
pub struct ConsensusRequirements {
    /// Whether majority voting is required
    pub requires_majority: bool,
    /// Minimum nodes needed (None means no minimum)
    pub min_nodes: Option<usize>,
    /// Description of the consensus mechanism
    pub description: String,
}

/// Consensus message for communication between nodes
/// 
/// This is a generic message type for the ConsensusAlgorithm trait.
/// Individual algorithms may use their own message types (e.g., PBFT uses PBFTMessage).
#[allow(dead_code)] // Reserved for future use in generic consensus implementations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMessage {
    pub algorithm: String,
    pub block_index: u64,
    pub block_hash: String,
    pub node_id: usize,
    pub data: Vec<u8>,
}
