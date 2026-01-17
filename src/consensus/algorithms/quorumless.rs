//! Quorum-less consensus with weighted voting

use crate::consensus::{
    ConsensusAlgorithm, ConsensusMessage, ConsensusRequirements, ConsensusResult,
};
use crate::etl::Block;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct NodeWeight {
    node_id: usize,
    weight: f64,
}

#[allow(dead_code)]
pub struct QuorumlessConsensus {
    node_id: usize,
    node_weights: Arc<RwLock<HashMap<usize, f64>>>,
    votes: Arc<RwLock<HashMap<u64, HashMap<usize, bool>>>>,
    committed: Arc<RwLock<HashSet<u64>>>,
    threshold_weight: f64,
}

impl QuorumlessConsensus {
    #[allow(dead_code)]
    pub fn new(node_id: usize, threshold_weight: f64) -> Self {
        let mut weights = HashMap::new();
        for i in 0..10 {
            weights.insert(i, 1.0);
        }

        Self {
            node_id,
            node_weights: Arc::new(RwLock::new(weights)),
            votes: Arc::new(RwLock::new(HashMap::new())),
            committed: Arc::new(RwLock::new(HashSet::new())),
            threshold_weight,
        }
    }

    #[allow(dead_code)]
    pub fn set_node_weight(&self, node_id: usize, weight: f64) {
        self.node_weights.write().insert(node_id, weight);
    }
}

#[async_trait]
impl ConsensusAlgorithm for QuorumlessConsensus {
    async fn propose(&self, block: &Block) -> Result<ConsensusResult, Box<dyn Error>> {
        let mut votes = self.votes.write();
        let block_votes = votes.entry(block.index).or_insert_with(HashMap::new);

        block_votes.insert(self.node_id, true);

        let weights = self.node_weights.read();
        let mut total_weight = 0.0;
        for (node_id, voted) in block_votes.iter() {
            if *voted {
                total_weight += weights.get(node_id).copied().unwrap_or(1.0);
            }
        }

        if total_weight >= self.threshold_weight {
            self.committed.write().insert(block.index);
            Ok(ConsensusResult::Committed(block.clone()))
        } else {
            Ok(ConsensusResult::Pending)
        }
    }

    async fn handle_message(
        &self,
        message: ConsensusMessage,
    ) -> Result<ConsensusResult, Box<dyn Error>> {
        {
            let mut votes = self.votes.write();
            let block_votes = votes
                .entry(message.block_index)
                .or_insert_with(HashMap::new);
            block_votes.insert(message.node_id, true);
        }
        Ok(ConsensusResult::Pending)
    }

    fn name(&self) -> &str {
        "Quorum-less (Weighted)"
    }

    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: false,
            min_nodes: None,
            description: format!(
                "Quorum-less weighted voting: threshold weight {}, no fixed majority",
                self.threshold_weight
            ),
        }
    }

    fn is_committed(&self, block_index: u64) -> bool {
        let committed = self.committed.read();
        committed.contains(&block_index)
    }
}
