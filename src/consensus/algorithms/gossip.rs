//! Gossip-based consensus

use crate::consensus::{
    ConsensusAlgorithm, ConsensusMessage, ConsensusRequirements, ConsensusResult,
};
use crate::etl::Block;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
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
    gossip_rounds: usize,
    fanout: usize,
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
            let gossip_state = state.entry(block.index).or_insert_with(|| GossipState {
                block_index: block.index,
                block_hash: block.hash.clone(),
                received_from: HashSet::new(),
                timestamp: Self::get_timestamp(),
            });
            gossip_state.received_from.insert(self.node_id);
        }

        for _ in 0..self.gossip_rounds {
            tokio::time::sleep(Duration::from_millis(100)).await;

            {
                let mut state = self.state.write();
                if let Some(gossip_state) = state.get_mut(&block.index) {
                    for _ in 0..self.fanout {
                        gossip_state.received_from.insert(self.node_id);
                    }
                }
            }
        }

        let state = self.state.read();
        if let Some(gossip_state) = state.get(&block.index) {
            if gossip_state.received_from.len() >= self.gossip_rounds {
                self.committed.write().insert(block.index);
                return Ok(ConsensusResult::Committed(block.clone()));
            }
        }

        Ok(ConsensusResult::Pending)
    }

    async fn handle_message(
        &self,
        message: ConsensusMessage,
    ) -> Result<ConsensusResult, Box<dyn Error>> {
        {
            let mut state = self.state.write();
            let gossip_state = state
                .entry(message.block_index)
                .or_insert_with(|| GossipState {
                    block_index: message.block_index,
                    block_hash: message.block_hash.clone(),
                    received_from: HashSet::new(),
                    timestamp: Self::get_timestamp(),
                });
            gossip_state.received_from.insert(message.node_id);
        }
        Ok(ConsensusResult::Pending)
    }

    fn name(&self) -> &str {
        "Gossip Protocol"
    }

    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: false,
            min_nodes: None,
            description: format!(
                "Gossip protocol: {} rounds, fanout={}, no majority voting",
                self.gossip_rounds, self.fanout
            ),
        }
    }

    fn is_committed(&self, block_index: u64) -> bool {
        let committed = self.committed.read();
        committed.contains(&block_index)
    }
}
