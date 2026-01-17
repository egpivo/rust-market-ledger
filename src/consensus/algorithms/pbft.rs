//! PBFT consensus implementation

use super::pbft_impl::PBFTManager;
use crate::consensus::{
    ConsensusAlgorithm, ConsensusMessage, ConsensusRequirements, ConsensusResult,
};
use crate::etl::Block;
use async_trait::async_trait;
use std::error::Error;
use std::sync::Arc;

pub struct PBFTConsensus {
    pbft: Arc<PBFTManager>,
    node_addresses: Vec<String>,
    port: u16,
}

impl PBFTConsensus {
    pub fn new(pbft: Arc<PBFTManager>, node_addresses: Vec<String>, port: u16) -> Self {
        Self {
            pbft,
            node_addresses,
            port,
        }
    }
}

#[async_trait]
impl ConsensusAlgorithm for PBFTConsensus {
    async fn propose(&self, block: &Block) -> Result<ConsensusResult, Box<dyn Error>> {
        use crate::network::broadcast_message;
        use std::time::Duration;

        let sequence = block.index;

        if self.pbft.is_primary(sequence) {
            let block_json = serde_json::to_string(block)?;
            let pre_prepare_msg = self
                .pbft
                .create_pre_prepare(&block.hash, &block_json, sequence);
            broadcast_message(&pre_prepare_msg, &self.node_addresses, self.port).await;
            self.pbft.handle_pre_prepare(&pre_prepare_msg);
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        let prepare_msg = self.pbft.create_prepare(&block.hash, sequence);
        broadcast_message(&prepare_msg, &self.node_addresses, self.port).await;
        self.pbft.handle_prepare(&prepare_msg);

        tokio::time::sleep(Duration::from_millis(500)).await;

        let commit_msg = self.pbft.create_commit(&block.hash, sequence);
        broadcast_message(&commit_msg, &self.node_addresses, self.port).await;
        self.pbft.handle_commit(&commit_msg);

        tokio::time::sleep(Duration::from_millis(500)).await;

        let state = self.pbft.state.read();
        if state.committed_blocks.contains(&sequence) {
            Ok(ConsensusResult::Committed(block.clone()))
        } else {
            Ok(ConsensusResult::Pending)
        }
    }

    async fn handle_message(
        &self,
        _message: ConsensusMessage,
    ) -> Result<ConsensusResult, Box<dyn Error>> {
        Ok(ConsensusResult::Pending)
    }

    fn name(&self) -> &str {
        "PBFT"
    }

    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: true,
            min_nodes: Some(4),
            description: "PBFT: Requires 2f+1 out of 3f+1 nodes (Byzantine fault tolerance)"
                .to_string(),
        }
    }

    fn is_committed(&self, block_index: u64) -> bool {
        let state = self.pbft.state.read();
        state.committed_blocks.contains(&block_index)
    }
}
