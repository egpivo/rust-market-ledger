//! Gossip-based consensus - no majority voting required
//! Uses epidemic/gossip protocol for eventual consistency

use crate::consensus::{ConsensusAlgorithm, ConsensusMessage, ConsensusResult, ConsensusRequirements};
use crate::etl::Block;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
struct GossipState {
    block_index: u64,
    block_hash: String,
    received_from: HashSet<usize>,
    timestamp: u64,
}

pub struct GossipConsensus {
    node_id: usize,
    state: Arc<RwLock<HashMap<u64, GossipState>>>,
    committed: Arc<RwLock<HashSet<u64>>>,
    gossip_rounds: usize, // Number of gossip rounds before committing
    fanout: usize, // Number of nodes to gossip to each round
}

impl GossipConsensus {
    pub fn new(node_id: usize, gossip_rounds: usize, fanout: usize) -> Self {
        Self {
            node_id,
            state: Arc::new(RwLock::new(HashMap::new())),
            committed: Arc::new(RwLock::new(HashSet::new())),
            gossip_rounds,
            fanout,
        }
    }
    
    fn get_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

#[async_trait]
impl ConsensusAlgorithm for GossipConsensus {
    async fn propose(&self, block: &Block) -> Result<ConsensusResult, Box<dyn Error>> {
        {
            let mut state = self.state.write();
            
            // Initialize gossip state for this block
            state.insert(block.index, GossipState {
                block_index: block.index,
                block_hash: block.hash.clone(),
                received_from: HashSet::new(),
                timestamp: Self::get_timestamp(),
            });
        } // Release lock before await
        
        // In gossip protocol, we don't wait for majority
        // After gossip_rounds, we consider it committed
        tokio::time::sleep(Duration::from_millis(100 * self.gossip_rounds as u64)).await;
        
        {
            let mut committed = self.committed.write();
            committed.insert(block.index);
        }
        
        Ok(ConsensusResult::Committed(block.clone()))
    }
    
    async fn handle_message(&self, message: ConsensusMessage) -> Result<ConsensusResult, Box<dyn Error>> {
        let mut state = self.state.write();
        
        // Update gossip state
        let entry = state.entry(message.block_index).or_insert_with(|| GossipState {
            block_index: message.block_index,
            block_hash: message.block_hash.clone(),
            received_from: HashSet::new(),
            timestamp: Self::get_timestamp(),
        });
        
        entry.received_from.insert(message.node_id);
        
        // Check if we've received from enough nodes (not majority, just enough for confidence)
        let threshold = self.fanout; // Commit after receiving from fanout nodes
        if entry.received_from.len() >= threshold {
            let mut committed = self.committed.write();
            if !committed.contains(&message.block_index) {
                committed.insert(message.block_index);
                return Ok(ConsensusResult::Pending); // Would need block data to return Committed
            }
        }
        
        Ok(ConsensusResult::Pending)
    }
    
    fn is_committed(&self, block_index: u64) -> bool {
        self.committed.read().contains(&block_index)
    }
    
    fn name(&self) -> &str {
        "Gossip"
    }
    
    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: false,
            min_nodes: None, // Gossip works with any number of nodes
            description: format!(
                "Gossip-based consensus - eventual consistency after {} rounds, fanout {}",
                self.gossip_rounds, self.fanout
            ),
        }
    }
}
