//! Consensus types and data structures

use crate::etl::Block;
use serde::{Deserialize, Serialize};

/// Consensus result
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMessage {
    pub algorithm: String,
    pub block_index: u64,
    pub block_hash: String,
    pub node_id: usize,
    pub data: Vec<u8>,
}
