//! Consensus algorithm trait definition

use crate::consensus::types::{ConsensusMessage, ConsensusResult, ConsensusRequirements};
use crate::etl::Block;
use async_trait::async_trait;
use std::error::Error;

/// Consensus algorithm trait - allows plugging in different consensus mechanisms
/// 
/// Note: This trait is defined for demonstration purposes and future extensibility.
/// Currently, main.rs uses PBFT directly, but this trait allows switching between
/// different consensus algorithms.
#[allow(dead_code)] // Reserved for future use or examples
#[async_trait]
pub trait ConsensusAlgorithm: Send + Sync {
    /// Propose a block for consensus
    async fn propose(&self, block: &Block) -> Result<ConsensusResult, Box<dyn Error>>;
    
    /// Handle incoming consensus message
    async fn handle_message(&self, message: ConsensusMessage) -> Result<ConsensusResult, Box<dyn Error>>;
    
    /// Check if a block has reached consensus
    fn is_committed(&self, block_index: u64) -> bool;
    
    /// Get the algorithm name
    fn name(&self) -> &str;
    
    /// Get consensus requirements (e.g., "majority", "all", "eventual", etc.)
    fn requirements(&self) -> ConsensusRequirements;
}
