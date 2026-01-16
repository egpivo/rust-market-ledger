//! Flexible Paxos consensus implementation
//! 
//! Flexible Paxos is a generalization of Paxos that relaxes the requirement
//! that all quorums in both phases must intersect. Instead, it only requires
//! that phase-1 (leader election) quorums intersect with previous phase-2
//! (acceptance) quorums. This allows more flexible quorum configurations.
//!
//! Key features:
//! - Phase-1 quorum (Q1) for leader election
//! - Phase-2 quorum (Q2) for value acceptance
//! - Q1 must intersect with any previous Q2 (safety requirement)
//! - Q2 quorums don't need to intersect with each other (flexibility)

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

/// Acceptor state for Flexible Paxos
#[derive(Clone, Debug)]
struct AcceptorState {
    promised: Option<ProposalId>,
    accepted: Option<(ProposalId, Block)>,
}

/// Flexible Paxos message types
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

/// Flexible Paxos consensus implementation
pub struct FlexiblePaxos {
    node_id: NodeId,
    total_nodes: usize,
    // Phase-1 quorum size (for leader election)
    q1_size: usize,
    // Phase-2 quorum size (for value acceptance)
    q2_size: usize,
    // Acceptor states (node_id -> state)
    acceptors: Arc<RwLock<HashMap<NodeId, AcceptorState>>>,
    // Proposer state
    current_proposal: Arc<RwLock<ProposalId>>,
    // Committed blocks
    committed: Arc<RwLock<HashSet<u64>>>,
    // Pending proposals
    pending_proposals: Arc<RwLock<HashMap<ProposalId, Block>>>,
}

impl FlexiblePaxos {
    /// Create a new Flexible Paxos instance
    /// 
    /// # Arguments
    /// * `node_id` - This node's ID
    /// * `total_nodes` - Total number of nodes in the cluster
    /// * `q1_size` - Phase-1 quorum size (must be >= majority for safety)
    /// * `q2_size` - Phase-2 quorum size (can be smaller than Q1 for flexibility)
    /// 
    /// # Safety Requirement
    /// Q1 must intersect with any previous Q2. Typically:
    /// - Q1 >= (total_nodes + 1) / 2 (majority)
    /// - Q2 can be smaller, but Q1 + Q2 > total_nodes (to ensure intersection)
    pub fn new(node_id: NodeId, total_nodes: usize, q1_size: usize, q2_size: usize) -> Self {
        // Safety check: Q1 + Q2 > total_nodes ensures intersection
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
            current_proposal: Arc::new(RwLock::new(node_id as ProposalId * 1000)),
            committed: Arc::new(RwLock::new(HashSet::new())),
            pending_proposals: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Generate a unique proposal number
    fn next_proposal(&self) -> ProposalId {
        let mut proposal = self.current_proposal.write();
        *proposal += self.total_nodes as ProposalId * 1000;
        *proposal
    }
    
    /// Check if a set of nodes forms a quorum
    fn is_quorum(&self, nodes: &HashSet<NodeId>, quorum_size: usize) -> bool {
        nodes.len() >= quorum_size
    }
    
    /// Handle Prepare message (Phase 1)
    fn handle_prepare(&self, from: NodeId, proposal: ProposalId) -> Option<FPaxosMessage> {
        let mut acceptors = self.acceptors.write();
        if let Some(acceptor) = acceptors.get_mut(&from) {
            let should_accept = match acceptor.promised {
                None => true,
                Some(p) => proposal > p,
            };
            
            if should_accept {
                acceptor.promised = Some(proposal);
                Some(FPaxosMessage::Promise {
                    from,
                    proposal,
                    accepted: acceptor.accepted.clone(),
                })
            } else {
                Some(FPaxosMessage::Reject {
                    from,
                    proposal,
                    reason: "Already promised to higher proposal".to_string(),
                })
            }
        } else {
            None
        }
    }
    
    /// Handle AcceptRequest message (Phase 2)
    fn handle_accept(&self, from: NodeId, proposal: ProposalId, value: Block) -> Option<FPaxosMessage> {
        let mut acceptors = self.acceptors.write();
        if let Some(acceptor) = acceptors.get_mut(&from) {
            let should_accept = match acceptor.promised {
                None => true,
                Some(p) => proposal >= p,
            };
            
            if should_accept {
                acceptor.promised = Some(proposal);
                acceptor.accepted = Some((proposal, value.clone()));
                Some(FPaxosMessage::Accepted {
                    from,
                    proposal,
                })
            } else {
                Some(FPaxosMessage::Reject {
                    from,
                    proposal,
                    reason: "Proposal number too low".to_string(),
                })
            }
        } else {
            None
        }
    }
}

#[async_trait]
impl ConsensusAlgorithm for FlexiblePaxos {
    async fn propose(&self, block: &Block) -> Result<ConsensusResult, Box<dyn Error>> {
        let proposal = self.next_proposal();
        
        // Store pending proposal
        {
            let mut pending = self.pending_proposals.write();
            pending.insert(proposal, block.clone());
        }
        
        // Phase 1: Prepare (Leader Election)
        let mut promises = HashSet::new();
        let mut highest_accepted: Option<(ProposalId, Block)> = None;
        
        // Simulate sending Prepare to all acceptors
        for node_id in 0..self.total_nodes {
            if let Some(response) = self.handle_prepare(node_id, proposal) {
                match response {
                    FPaxosMessage::Promise { from, proposal: _p, accepted } => {
                        promises.insert(from);
                        if let Some((prop_id, value)) = accepted {
                            if highest_accepted.is_none() || prop_id > highest_accepted.as_ref().unwrap().0 {
                                highest_accepted = Some((prop_id, value));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Check if we have Q1 quorum
        if !self.is_quorum(&promises, self.q1_size) {
            return Ok(ConsensusResult::Pending);
        }
        
        // Phase 2: Accept (Value Acceptance)
        // Use the highest accepted value if any, otherwise use the new block
        let value_to_accept = if let Some((_, accepted_block)) = highest_accepted {
            accepted_block
        } else {
            block.clone()
        };
        
        let mut accepted = HashSet::new();
        
        // Simulate sending AcceptRequest to all acceptors
        for node_id in 0..self.total_nodes {
            if let Some(response) = self.handle_accept(node_id, proposal, value_to_accept.clone()) {
                match response {
                    FPaxosMessage::Accepted { from, proposal: p } => {
                        if p == proposal {
                            accepted.insert(from);
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Check if we have Q2 quorum
        if self.is_quorum(&accepted, self.q2_size) {
            // Commit the block
            {
                let mut committed = self.committed.write();
                committed.insert(block.index);
            }
            
            // Clean up pending proposal
            {
                let mut pending = self.pending_proposals.write();
                pending.remove(&proposal);
            }
            
            Ok(ConsensusResult::Committed(value_to_accept))
        } else {
            Ok(ConsensusResult::Pending)
        }
    }
    
    async fn handle_message(&self, _message: ConsensusMessage) -> Result<ConsensusResult, Box<dyn Error>> {
        // In a full implementation, this would handle network messages
        // For now, we simulate everything locally
        Ok(ConsensusResult::Pending)
    }
    
    fn is_committed(&self, block_index: u64) -> bool {
        let committed = self.committed.read();
        committed.contains(&block_index)
    }
    
    fn name(&self) -> &str {
        "Flexible Paxos"
    }
    
    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: true, // Q1 requires majority
            min_nodes: Some(self.q1_size),
            description: format!(
                "Flexible Paxos with Q1={} (phase-1) and Q2={} (phase-2) quorums. Q1 must intersect with previous Q2.",
                self.q1_size, self.q2_size
            ),
        }
    }
}
