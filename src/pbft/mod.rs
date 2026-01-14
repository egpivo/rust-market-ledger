use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// PBFT 訊息類型
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MessageType {
    PrePrepare,
    Prepare,
    Commit,
}

/// PBFT 訊息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PBFTMessage {
    pub msg_type: MessageType,
    pub view: u64,           // 當前視圖編號
    pub sequence: u64,       // 序列號（區塊索引）
    pub block_hash: String,  // 區塊哈希
    pub block_data_json: Option<String>, // 只在 PrePrepare 時包含（序列化的區塊數據）
    pub node_id: usize,      // 發送節點 ID
    pub timestamp: i64,
}

/// PBFT 節點狀態
#[derive(Debug, Clone)]
pub struct NodeState {
    pub node_id: usize,
    pub view: u64,
    pub sequence: u64,
    pub pre_prepares: HashMap<(u64, u64), Vec<usize>>, // (view, sequence) -> node_ids
    pub prepares: HashMap<(u64, u64), Vec<usize>>,
    pub commits: HashMap<(u64, u64), Vec<usize>>,
    pub committed_blocks: Vec<u64>, // 已提交的區塊索引
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

    /// 計算法定人數 (2f + 1，其中 f 是容錯節點數)
    pub fn quorum_size(&self, total_nodes: usize) -> usize {
        let f = (total_nodes - 1) / 3; // 最多容忍 f 個拜占庭節點
        (2 * f) + 1
    }

    /// 檢查是否達到法定人數
    pub fn has_quorum(&self, votes: &[usize], total_nodes: usize) -> bool {
        votes.len() >= self.quorum_size(total_nodes)
    }
}

/// PBFT 共識管理器
pub struct PBFTManager {
    pub state: Arc<RwLock<NodeState>>,
    pub total_nodes: usize,
    pub node_addresses: Vec<String>, // 所有節點的地址列表
}

impl PBFTManager {
    pub fn new(node_id: usize, total_nodes: usize, node_addresses: Vec<String>) -> Self {
        PBFTManager {
            state: Arc::new(RwLock::new(NodeState::new(node_id))),
            total_nodes,
            node_addresses,
        }
    }

    /// 處理 PrePrepare 訊息
    pub fn handle_pre_prepare(&self, msg: &PBFTMessage) -> bool {
        let mut state = self.state.write();
        let key = (msg.view, msg.sequence);
        
        let votes = state.pre_prepares.entry(key).or_insert_with(Vec::new);
        if !votes.contains(&msg.node_id) {
            votes.push(msg.node_id);
        }
        
        state.has_quorum(votes, self.total_nodes)
    }

    /// 處理 Prepare 訊息
    pub fn handle_prepare(&self, msg: &PBFTMessage) -> bool {
        let mut state = self.state.write();
        let key = (msg.view, msg.sequence);
        
        let votes = state.prepares.entry(key).or_insert_with(Vec::new);
        if !votes.contains(&msg.node_id) {
            votes.push(msg.node_id);
        }
        
        state.has_quorum(votes, self.total_nodes)
    }

    /// 處理 Commit 訊息
    pub fn handle_commit(&self, msg: &PBFTMessage) -> bool {
        let mut state = self.state.write();
        let key = (msg.view, msg.sequence);
        
        let votes = state.commits.entry(key).or_insert_with(Vec::new);
        if !votes.contains(&msg.node_id) {
            votes.push(msg.node_id);
        }
        
        let has_quorum = state.has_quorum(votes, self.total_nodes);
        if has_quorum && !state.committed_blocks.contains(&msg.sequence) {
            state.committed_blocks.push(msg.sequence);
        }
        has_quorum
    }

    /// 檢查區塊是否已提交
    pub fn is_committed(&self, sequence: u64) -> bool {
        let state = self.state.read();
        state.committed_blocks.contains(&sequence)
    }

    /// 獲取當前節點 ID
    pub fn node_id(&self) -> usize {
        self.state.read().node_id
    }

    /// 創建 PrePrepare 訊息（主節點發起）
    /// block_hash: 區塊哈希
    /// block_data_json: 序列化的區塊數據（JSON 字串）
    pub fn create_pre_prepare(&self, block_hash: &str, block_data_json: &str, sequence: u64) -> PBFTMessage {
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

    /// 創建 Prepare 訊息
    pub fn create_prepare(&self, block_hash: &str, sequence: u64) -> PBFTMessage {
        let state = self.state.read();
        PBFTMessage {
            msg_type: MessageType::Prepare,
            view: state.view,
            sequence,
            block_hash: block_hash.to_string(),
            block_data: None,
            node_id: state.node_id,
            timestamp: Utc::now().timestamp(),
        }
    }

    /// 創建 Commit 訊息
    pub fn create_commit(&self, block_hash: &str, sequence: u64) -> PBFTMessage {
        let state = self.state.read();
        PBFTMessage {
            msg_type: MessageType::Commit,
            view: state.view,
            sequence,
            block_hash: block_hash.to_string(),
            block_data: None,
            node_id: state.node_id,
            timestamp: Utc::now().timestamp(),
        }
    }

    /// 判斷是否為主節點（簡化版：輪流擔任）
    pub fn is_primary(&self, sequence: u64) -> bool {
        (sequence % self.total_nodes as u64) as usize == self.node_id()
    }
}
