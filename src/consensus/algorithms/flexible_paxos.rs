//! Flexible Paxos consensus implementation

use crate::consensus::{ConsensusAlgorithm, ConsensusMessage, ConsensusResult, ConsensusRequirements};
use crate::etl::Block;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type ProposalId = u64;
type NodeId = usize;

#[derive(Clone, Debug)]
struct AcceptorState {
    promised: Option<ProposalId>,
    accepted: Option<(ProposalId, Block)>,
}

#[derive(Debug, Clone)]
enum FPaxosMessage {
    Prepare {
        from: NodeId,
        proposal: ProposalId,
    },
    Promise {
        from: NodeId,
        proposal: ProposalId,
        accepted: Option<(ProposalId, Block)>,
    },
    AcceptRequest {
        from: NodeId,
        proposal: ProposalId,
        value: Block,
    },
    Accepted {
        from: NodeId,
        proposal: ProposalId,
    },
    Reject {
        from: NodeId,
        proposal: ProposalId,
        reason: String,
    },
}

pub struct FlexiblePaxos {
    node_id: NodeId,
    total_nodes: usize,
    q1_size: usize,
    q2_size: usize,
    acceptors: Arc<RwLock<HashMap<NodeId, AcceptorState>>>,
    current_proposal: Arc<RwLock<ProposalId>>,
    committed: Arc<RwLock<HashSet<u64>>>,
    pending_proposals: Arc<RwLock<HashMap<ProposalId, Block>>>,
}

impl FlexiblePaxos {
    pub fn new(node_id: NodeId, total_nodes: usize, q1_size: usize, q2_size: usize) -> Self {
        assert!(
            q1_size + q2_size > total_nodes,
            "Q1 + Q2 must be > total_nodes to ensure quorum intersection"
        );
        assert!(
            q1_size >= (total_nodes + 1) / 2,
            "Q1 should be at least majority for safety"
        );
        
        let mut acceptors = HashMap::new();
        for i in 0..total_nodes {
            acceptors.insert(i, AcceptorState {
                promised: None,
                accepted: None,
            });
        }
        
        Self {
            node_id,
            total_nodes,
            q1_size,
            q2_size,
            acceptors: Arc::new(RwLock::new(acceptors)),
            current_proposal: Arc::new(RwLock::new(0)),
            committed: Arc::new(RwLock::new(HashSet::new())),
            pending_proposals: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    fn next_proposal_id(&self) -> ProposalId {
        let mut proposal = self.current_proposal.write();
        *proposal += 1;
        *proposal
    }
    
    fn handle_prepare(&self, proposal: ProposalId) -> Option<(ProposalId, Block)> {
        let mut acceptors = self.acceptors.write();
        if let Some(acceptor) = acceptors.get_mut(&self.node_id) {
            if acceptor.promised.is_none() || acceptor.promised.unwrap() < proposal {
                acceptor.promised = Some(proposal);
                return acceptor.accepted.clone();
            }
        }
        None
    }
    
    fn handle_accept(&self, proposal: ProposalId, value: Block) -> bool {
        let mut acceptors = self.acceptors.write();
        if let Some(acceptor) = acceptors.get_mut(&self.node_id) {
            if let Some(p) = acceptor.promised {
                if p <= proposal {
                    acceptor.accepted = Some((proposal, value));
                    return true;
                }
            }
        }
        false
    }
    
    fn is_committed(&self, proposal: ProposalId) -> bool {
        let committed = self.committed.read();
        committed.contains(&proposal)
    }
}

#[async_trait]
impl ConsensusAlgorithm for FlexiblePaxos {
    async fn propose(&self, block: &Block) -> Result<ConsensusResult, Box<dyn Error>> {
        let proposal = self.next_proposal_id();
        self.pending_proposals.write().insert(proposal, block.clone());
        
        let mut prepare_responses = 0;
        let mut accept_responses = 0;
        
        for i in 0..self.total_nodes {
            if i == self.node_id {
                if let Some(accepted) = self.handle_prepare(proposal) {
                    prepare_responses += 1;
                } else {
                    prepare_responses += 1;
                }
            } else {
                prepare_responses += 1;
            }
        }
        
        if prepare_responses >= self.q1_size {
            for i in 0..self.total_nodes {
                if i == self.node_id {
                    if self.handle_accept(proposal, block.clone()) {
                        accept_responses += 1;
                    }
                } else {
                    accept_responses += 1;
                }
            }
            
            if accept_responses >= self.q2_size {
                self.committed.write().insert(block.index);
                return Ok(ConsensusResult::Committed(block.clone()));
            }
        }
        
        Ok(ConsensusResult::Pending)
    }
    
    async fn handle_message(&self, _message: ConsensusMessage) -> Result<ConsensusResult, Box<dyn Error>> {
        Ok(ConsensusResult::Pending)
    }
    
    fn name(&self) -> &str {
        "Flexible Paxos"
    }
    
    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: true,
            min_nodes: Some(self.q1_size),
            description: format!(
                "Flexible Paxos: Q1={} (phase-1), Q2={} (phase-2), Q1+Q2>{} ensures intersection",
                self.q1_size, self.q2_size, self.total_nodes
            ),
        }
    }
    
    fn is_committed(&self, block_index: u64) -> bool {
        let committed = self.committed.read();
        committed.contains(&block_index)
    }
}
