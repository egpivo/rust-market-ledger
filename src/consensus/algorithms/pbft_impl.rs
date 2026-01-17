//! PBFT implementation details
//! This module contains the core PBFT logic (PBFTManager, PBFTMessage, etc.)

use chrono::prelude::*;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MessageType {
    PrePrepare,
    Prepare,
    Commit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PBFTMessage {
    pub msg_type: MessageType,
    pub view: u64,
    pub sequence: u64,
    pub block_hash: String,
    pub block_data_json: Option<String>,
    pub node_id: usize,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct NodeState {
    pub node_id: usize,
    pub view: u64,
    pub sequence: u64,
    pub pre_prepares: HashMap<(u64, u64), Vec<usize>>,
    pub prepares: HashMap<(u64, u64), Vec<usize>>,
    pub commits: HashMap<(u64, u64), Vec<usize>>,
    pub committed_blocks: Vec<u64>,
}

impl NodeState {
    pub fn new(node_id: usize) -> Self {
        NodeState {
            node_id,
            view: 0,
            sequence: 0,
            pre_prepares: HashMap::new(),
            prepares: HashMap::new(),
            commits: HashMap::new(),
            committed_blocks: Vec::new(),
        }
    }

    pub fn quorum_size(&self, total_nodes: usize) -> usize {
        let f = (total_nodes - 1) / 3;
        (2 * f) + 1
    }

    pub fn has_quorum(&self, votes: &[usize], total_nodes: usize) -> bool {
        votes.len() >= self.quorum_size(total_nodes)
    }
}

pub struct PBFTManager {
    pub state: Arc<RwLock<NodeState>>,
    pub total_nodes: usize,
    pub node_addresses: Vec<String>,
}

impl PBFTManager {
    pub fn new(node_id: usize, total_nodes: usize, node_addresses: Vec<String>) -> Self {
        PBFTManager {
            state: Arc::new(RwLock::new(NodeState::new(node_id))),
            total_nodes,
            node_addresses,
        }
    }

    pub fn handle_pre_prepare(&self, msg: &PBFTMessage) -> bool {
        let key = (msg.view, msg.sequence);
        let total_nodes = self.total_nodes;

        {
            let mut state = self.state.write();
            let votes = state.pre_prepares.entry(key).or_insert_with(Vec::new);
            if !votes.contains(&msg.node_id) {
                votes.push(msg.node_id);
            }
        }

        let state = self.state.read();
        let votes = state.pre_prepares.get(&key).unwrap();
        state.has_quorum(votes, total_nodes)
    }

    pub fn handle_prepare(&self, msg: &PBFTMessage) -> bool {
        let key = (msg.view, msg.sequence);
        let total_nodes = self.total_nodes;

        {
            let mut state = self.state.write();
            let votes = state.prepares.entry(key).or_insert_with(Vec::new);
            if !votes.contains(&msg.node_id) {
                votes.push(msg.node_id);
            }
        }

        let state = self.state.read();
        let votes = state.prepares.get(&key).unwrap();
        state.has_quorum(votes, total_nodes)
    }

    pub fn handle_commit(&self, msg: &PBFTMessage) -> bool {
        let key = (msg.view, msg.sequence);
        let total_nodes = self.total_nodes;
        let sequence = msg.sequence;

        {
            let mut state = self.state.write();
            let votes = state.commits.entry(key).or_insert_with(Vec::new);
            if !votes.contains(&msg.node_id) {
                votes.push(msg.node_id);
            }
        }

        let mut state = self.state.write();
        let votes = state.commits.get(&key).unwrap();
        let has_quorum = state.has_quorum(votes, total_nodes);
        if has_quorum && !state.committed_blocks.contains(&sequence) {
            state.committed_blocks.push(sequence);
        }
        has_quorum
    }

    pub fn is_committed(&self, sequence: u64) -> bool {
        let state = self.state.read();
        state.committed_blocks.contains(&sequence)
    }

    pub fn node_id(&self) -> usize {
        self.state.read().node_id
    }

    pub fn create_pre_prepare(
        &self,
        block_hash: &str,
        block_data_json: &str,
        sequence: u64,
    ) -> PBFTMessage {
        let state = self.state.read();
        PBFTMessage {
            msg_type: MessageType::PrePrepare,
            view: state.view,
            sequence,
            block_hash: block_hash.to_string(),
            block_data_json: Some(block_data_json.to_string()),
            node_id: state.node_id,
            timestamp: Utc::now().timestamp(),
        }
    }

    pub fn create_prepare(&self, block_hash: &str, sequence: u64) -> PBFTMessage {
        let state = self.state.read();
        PBFTMessage {
            msg_type: MessageType::Prepare,
            view: state.view,
            sequence,
            block_hash: block_hash.to_string(),
            block_data_json: None,
            node_id: state.node_id,
            timestamp: Utc::now().timestamp(),
        }
    }

    pub fn create_commit(&self, block_hash: &str, sequence: u64) -> PBFTMessage {
        let state = self.state.read();
        PBFTMessage {
            msg_type: MessageType::Commit,
            view: state.view,
            sequence,
            block_hash: block_hash.to_string(),
            block_data_json: None,
            node_id: state.node_id,
            timestamp: Utc::now().timestamp(),
        }
    }

    pub fn is_primary(&self, sequence: u64) -> bool {
        (sequence % self.total_nodes as u64) as usize == self.node_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Initialize logger for tests (only once)
    static INIT: std::sync::Once = std::sync::Once::new();

    fn init() {
        INIT.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error")),
                )
                .with_test_writer()
                .try_init();
        });
    }

    #[test]
    fn test_quorum_size_calculation() {
        init();
        let state = NodeState::new(0);

        assert_eq!(state.quorum_size(4), 3);
        assert_eq!(state.quorum_size(7), 5);
        assert_eq!(state.quorum_size(10), 7);
    }

    #[test]
    fn test_has_quorum() {
        init();
        let state = NodeState::new(0);

        assert!(state.has_quorum(&[0, 1, 2], 4));
        assert!(!state.has_quorum(&[0, 1], 4));
        assert!(state.has_quorum(&[0, 1, 2, 3], 4));
    }

    #[test]
    fn test_pbft_manager_creation() {
        init();
        let addresses = vec!["127.0.0.1:8000".to_string(), "127.0.0.1:8001".to_string()];
        let manager = PBFTManager::new(0, 2, addresses);

        assert_eq!(manager.node_id(), 0);
        assert_eq!(manager.total_nodes, 2);
    }

    #[test]
    fn test_is_primary() {
        init();
        let addresses = vec![
            "127.0.0.1:8000".to_string(),
            "127.0.0.1:8001".to_string(),
            "127.0.0.1:8002".to_string(),
        ];

        let manager0 = PBFTManager::new(0, 3, addresses.clone());
        let manager1 = PBFTManager::new(1, 3, addresses.clone());
        let manager2 = PBFTManager::new(2, 3, addresses);

        assert!(manager0.is_primary(0));
        assert!(manager1.is_primary(1));
        assert!(manager2.is_primary(2));
        assert!(manager0.is_primary(3));
    }

    #[test]
    fn test_message_handling() {
        init();
        let addresses = vec![
            "127.0.0.1:8000".to_string(),
            "127.0.0.1:8001".to_string(),
            "127.0.0.1:8002".to_string(),
            "127.0.0.1:8003".to_string(),
        ];
        let manager = PBFTManager::new(0, 4, addresses);

        let msg = PBFTMessage {
            msg_type: MessageType::Prepare,
            view: 0,
            sequence: 1,
            block_hash: "test_hash".to_string(),
            block_data_json: None,
            node_id: 1,
            timestamp: 1234567890,
        };

        let result = manager.handle_prepare(&msg);
        assert!(!result);
    }

    #[test]
    fn test_quorum_reached() {
        init();
        let addresses = vec![
            "127.0.0.1:8000".to_string(),
            "127.0.0.1:8001".to_string(),
            "127.0.0.1:8002".to_string(),
            "127.0.0.1:8003".to_string(),
        ];
        let manager = PBFTManager::new(0, 4, addresses);

        let msg1 = PBFTMessage {
            msg_type: MessageType::Commit,
            view: 0,
            sequence: 1,
            block_hash: "test_hash".to_string(),
            block_data_json: None,
            node_id: 0,
            timestamp: 1234567890,
        };

        let msg2 = PBFTMessage {
            msg_type: MessageType::Commit,
            view: 0,
            sequence: 1,
            block_hash: "test_hash".to_string(),
            block_data_json: None,
            node_id: 1,
            timestamp: 1234567890,
        };

        let msg3 = PBFTMessage {
            msg_type: MessageType::Commit,
            view: 0,
            sequence: 1,
            block_hash: "test_hash".to_string(),
            block_data_json: None,
            node_id: 2,
            timestamp: 1234567890,
        };

        manager.handle_commit(&msg1);
        manager.handle_commit(&msg2);
        let result = manager.handle_commit(&msg3);

        assert!(result);
        assert!(manager.is_committed(1));
    }
}
