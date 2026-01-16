//! PBFT (Practical Byzantine Fault Tolerance) consensus implementation
//! Requires majority voting: 2f+1 nodes out of 3f+1 total nodes

use crate::consensus::{ConsensusAlgorithm, ConsensusMessage, ConsensusResult, ConsensusRequirements};
use crate::etl::Block;
use super::pbft_impl::PBFTManager;
use async_trait::async_trait;
use std::error::Error;
use std::sync::Arc;

pub struct PBFTConsensus {
    pbft: Arc<PBFTManager>,
    node_addresses: Vec<String>,
    port: u16,
}

impl PBFTConsensus {
    pub fn new(
        pbft: Arc<PBFTManager>,
        node_addresses: Vec<String>,
        port: u16,
    ) -> Self {
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
        
        // Pre-Prepare phase
        if self.pbft.is_primary(sequence) {
            let block_json = serde_json::to_string(block)?;
            let pre_prepare_msg = self.pbft.create_pre_prepare(&block.hash, &block_json, sequence);
            broadcast_message(&pre_prepare_msg, &self.node_addresses, self.port).await;
            self.pbft.handle_pre_prepare(&pre_prepare_msg);
        }
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Prepare phase
        let prepare_msg = self.pbft.create_prepare(&block.hash, sequence);
        broadcast_message(&prepare_msg, &self.node_addresses, self.port).await;
        let prepare_quorum = self.pbft.handle_prepare(&prepare_msg);
        
        if !prepare_quorum {
            return Ok(ConsensusResult::Pending);
        }
        
        // Commit phase
        let commit_msg = self.pbft.create_commit(&block.hash, sequence);
        broadcast_message(&commit_msg, &self.node_addresses, self.port).await;
        let commit_quorum = self.pbft.handle_commit(&commit_msg);
        
        if commit_quorum {
            Ok(ConsensusResult::Committed(block.clone()))
        } else {
            Ok(ConsensusResult::Pending)
        }
    }
    
    async fn handle_message(&self, _message: ConsensusMessage) -> Result<ConsensusResult, Box<dyn Error>> {
        // PBFT handles messages through its own message system
        // This is a placeholder - in practice, PBFT messages come through the network layer
        Ok(ConsensusResult::Pending)
    }
    
    fn is_committed(&self, block_index: u64) -> bool {
        self.pbft.is_committed(block_index)
    }
    
    fn name(&self) -> &str {
        "PBFT"
    }
    
    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: true,
            min_nodes: Some(4), // PBFT requires at least 4 nodes (3f+1, f>=1)
            description: "Practical Byzantine Fault Tolerance - requires 2f+1 out of 3f+1 nodes".to_string(),
        }
    }
}
