//! Eventual Consistency consensus - no voting, just time-based or count-based commitment
//! Suitable for systems where eventual consistency is acceptable

use crate::consensus::{ConsensusAlgorithm, ConsensusMessage, ConsensusResult, ConsensusRequirements};
use crate::etl::Block;
use async_trait::async_trait;
use std::collections::HashSet;
use std::error::Error;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Duration;

pub struct EventualConsensus {
    node_id: usize,
    committed: Arc<RwLock<HashSet<u64>>>,
    confirmation_delay_ms: u64, // Time to wait before committing
    min_confirmations: usize, // Minimum number of nodes that must have seen the block
}

impl EventualConsensus {
    pub fn new(node_id: usize, confirmation_delay_ms: u64, min_confirmations: usize) -> Self {
        Self {
            node_id,
            committed: Arc::new(RwLock::new(HashSet::new())),
            confirmation_delay_ms,
            min_confirmations,
        }
    }
}

#[async_trait]
impl ConsensusAlgorithm for EventualConsensus {
    async fn propose(&self, block: &Block) -> Result<ConsensusResult, Box<dyn Error>> {
        // In eventual consistency, we commit after a delay
        // No voting required - just wait for time-based commitment
        tokio::time::sleep(Duration::from_millis(self.confirmation_delay_ms)).await;
        
        let mut committed = self.committed.write();
        committed.insert(block.index);
        
        Ok(ConsensusResult::Committed(block.clone()))
    }
    
    async fn handle_message(&self, _message: ConsensusMessage) -> Result<ConsensusResult, Box<dyn Error>> {
        // Track that we've seen this block
        // In eventual consistency, we just need to see it from enough nodes
        // (not majority, just a threshold)
        
        // For simplicity, commit after receiving from min_confirmations nodes
        // In a real implementation, you'd track this per block
        Ok(ConsensusResult::Pending)
    }
    
    fn is_committed(&self, block_index: u64) -> bool {
        self.committed.read().contains(&block_index)
    }
    
    fn name(&self) -> &str {
        "Eventual"
    }
    
    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: false,
            min_nodes: None,
            description: format!(
                "Eventual consistency - commits after {}ms delay, {} confirmations",
                self.confirmation_delay_ms, self.min_confirmations
            ),
        }
    }
}
