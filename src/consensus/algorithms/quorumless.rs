//! Quorum-less consensus - uses weighted voting or reputation-based system
//! No fixed majority required, uses dynamic thresholds

use crate::consensus::{ConsensusAlgorithm, ConsensusMessage, ConsensusResult, ConsensusRequirements};
use crate::etl::Block;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Clone, Debug)]
#[allow(dead_code)] // Reserved for future use or examples
struct NodeWeight {
    node_id: usize,
    weight: f64, // Reputation/weight of the node
}

/// Quorum-less consensus with weighted voting
/// 
/// Note: This is implemented but not currently used in main.rs.
/// It's available for demonstration and future use.
#[allow(dead_code)] // Reserved for future use or examples
pub struct QuorumlessConsensus {
    node_id: usize,
    node_weights: Arc<RwLock<HashMap<usize, f64>>>, // Node ID -> Weight
    votes: Arc<RwLock<HashMap<u64, HashMap<usize, bool>>>>, // Block index -> (Node ID -> Vote)
    committed: Arc<RwLock<HashSet<u64>>>,
    threshold_weight: f64, // Total weight threshold (not count threshold)
}

impl QuorumlessConsensus {
    #[allow(dead_code)] // Reserved for future use or examples
    pub fn new(node_id: usize, threshold_weight: f64) -> Self {
        let mut weights = HashMap::new();
        // Initialize all nodes with equal weight (can be customized)
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
    
    #[allow(dead_code)] // Reserved for future use or examples
    pub fn set_node_weight(&self, node_id: usize, weight: f64) {
        self.node_weights.write().insert(node_id, weight);
    }
    
    #[allow(dead_code)] // Reserved for future use or examples
    fn calculate_total_weight(&self, block_index: u64) -> f64 {
        let votes = self.votes.read();
        let weights = self.node_weights.read();
        
        votes
            .get(&block_index)
            .map(|block_votes| {
                block_votes
                    .iter()
                    .filter(|(_, &voted)| voted)
                    .map(|(node_id, _)| weights.get(node_id).copied().unwrap_or(0.0))
                    .sum()
            })
            .unwrap_or(0.0)
    }
}

#[async_trait]
impl ConsensusAlgorithm for QuorumlessConsensus {
    async fn propose(&self, block: &Block) -> Result<ConsensusResult, Box<dyn Error>> {
        // Record our vote
        let mut votes = self.votes.write();
        let block_votes = votes.entry(block.index).or_insert_with(HashMap::new);
        block_votes.insert(self.node_id, true);
        
        // Check if threshold weight is reached
        let total_weight = self.calculate_total_weight(block.index);
        
        if total_weight >= self.threshold_weight {
            let mut committed = self.committed.write();
            committed.insert(block.index);
            Ok(ConsensusResult::Committed(block.clone()))
        } else {
            Ok(ConsensusResult::Pending)
        }
    }
    
    async fn handle_message(&self, message: ConsensusMessage) -> Result<ConsensusResult, Box<dyn Error>> {
        // Record vote from other node
        let mut votes = self.votes.write();
        let block_votes = votes.entry(message.block_index).or_insert_with(HashMap::new);
        block_votes.insert(message.node_id, true);
        
        // Check if threshold weight is reached
        let total_weight = self.calculate_total_weight(message.block_index);
        
        if total_weight >= self.threshold_weight {
            let mut committed = self.committed.write();
            if !committed.contains(&message.block_index) {
                committed.insert(message.block_index);
                return Ok(ConsensusResult::Pending); // Would need block data
            }
        }
        
        Ok(ConsensusResult::Pending)
    }
    
    fn is_committed(&self, block_index: u64) -> bool {
        self.committed.read().contains(&block_index)
    }
    
    fn name(&self) -> &str {
        "Quorumless"
    }
    
    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: false,
            min_nodes: None,
            description: format!(
                "Weighted voting consensus - requires {} total weight (not majority count)",
                self.threshold_weight
            ),
        }
    }
}
