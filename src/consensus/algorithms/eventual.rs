//! Eventual Consistency consensus

use crate::consensus::{
    ConsensusAlgorithm, ConsensusMessage, ConsensusRequirements, ConsensusResult,
};
use crate::etl::Block;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

pub struct EventualConsensus {
    node_id: usize,
    committed: Arc<RwLock<HashSet<u64>>>,
    confirmation_delay_ms: u64,
    min_confirmations: usize,
}

impl EventualConsensus {
    #[allow(dead_code)]
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
        tokio::time::sleep(Duration::from_millis(self.confirmation_delay_ms)).await;

        let mut committed = self.committed.write();
        committed.insert(block.index);

        Ok(ConsensusResult::Committed(block.clone()))
    }

    async fn handle_message(
        &self,
        _message: ConsensusMessage,
    ) -> Result<ConsensusResult, Box<dyn Error>> {
        Ok(ConsensusResult::Pending)
    }

    fn name(&self) -> &str {
        "Eventual Consistency"
    }

    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: false,
            min_nodes: None,
            description: format!(
                "Eventual consistency: {}ms delay, {} min confirmations, no majority voting",
                self.confirmation_delay_ms, self.min_confirmations
            ),
        }
    }

    fn is_committed(&self, block_index: u64) -> bool {
        let committed = self.committed.read();
        committed.contains(&block_index)
    }
}
