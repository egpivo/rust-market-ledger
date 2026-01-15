//! Examples of using different consensus algorithms
//! 
//! This module demonstrates how to use various consensus mechanisms,
//! including those that don't require majority voting.

use crate::consensus::*;
use crate::consensus::algorithms::*;
use crate::etl::{Block, MarketData};
use std::sync::Arc;

/// Example: Compare different consensus algorithms
pub async fn compare_consensus_algorithms() {
    println!("\n=== Consensus Algorithm Comparison ===\n");
    
    // Create a test block
    let test_block = Block {
        index: 1,
        timestamp: chrono::Utc::now().timestamp(),
        data: vec![MarketData {
            asset: "BTC".to_string(),
            price: 50000.0,
            source: "Test".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }],
        previous_hash: "0000_genesis".to_string(),
        hash: "test_hash".to_string(),
        nonce: 0,
    };
    
    // 1. PBFT (requires majority)
    println!("1. PBFT Consensus:");
    println!("   - Requires majority voting: YES");
    println!("   - Minimum nodes: 4 (3f+1, f>=1)");
    println!("   - Quorum: 2f+1 out of 3f+1 nodes");
    println!("   - Use case: Byzantine fault tolerance with strong consistency\n");
    
    // 2. Gossip (no majority)
    let gossip = Arc::new(gossip::GossipConsensus::new(0, 3, 2));
    println!("2. Gossip Consensus:");
    println!("   - Requires majority voting: NO");
    println!("   - Requirements: {}", gossip.requirements().description);
    println!("   - Use case: Large-scale systems, eventual consistency\n");
    
    // 3. Eventual Consistency (no majority)
    let eventual = Arc::new(eventual::EventualConsensus::new(0, 1000, 2));
    println!("3. Eventual Consistency:");
    println!("   - Requires majority voting: NO");
    println!("   - Requirements: {}", eventual.requirements().description);
    println!("   - Use case: Systems where eventual consistency is acceptable\n");
    
    // 4. Quorum-less (weighted voting)
    let quorumless = Arc::new(quorumless::QuorumlessConsensus::new(0, 5.0));
    println!("4. Quorum-less (Weighted) Consensus:");
    println!("   - Requires majority voting: NO");
    println!("   - Requirements: {}", quorumless.requirements().description);
    println!("   - Use case: Reputation-based systems, weighted voting\n");
}

/// Example: Test gossip consensus
pub async fn test_gossip_consensus() {
    println!("\n=== Testing Gossip Consensus ===\n");
    
    let consensus = Arc::new(gossip::GossipConsensus::new(0, 3, 2));
    
    let block = Block {
        index: 1,
        timestamp: chrono::Utc::now().timestamp(),
        data: vec![MarketData {
            asset: "BTC".to_string(),
            price: 50000.0,
            source: "Test".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }],
        previous_hash: "0000_genesis".to_string(),
        hash: "test_hash".to_string(),
        nonce: 0,
    };
    
    println!("Proposing block with Gossip consensus...");
    let result = consensus.propose(&block).await.unwrap();
    
    match result {
        ConsensusResult::Committed(_) => println!("✓ Block committed!"),
        ConsensusResult::Pending => println!("⏳ Block pending..."),
        ConsensusResult::Rejected(reason) => println!("✗ Block rejected: {}", reason),
    }
}

/// Example: Test eventual consistency
pub async fn test_eventual_consensus() {
    println!("\n=== Testing Eventual Consistency ===\n");
    
    let consensus = Arc::new(eventual::EventualConsensus::new(0, 500, 2));
    
    let block = Block {
        index: 1,
        timestamp: chrono::Utc::now().timestamp(),
        data: vec![MarketData {
            asset: "BTC".to_string(),
            price: 50000.0,
            source: "Test".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }],
        previous_hash: "0000_genesis".to_string(),
        hash: "test_hash".to_string(),
        nonce: 0,
    };
    
    println!("Proposing block with Eventual consensus (500ms delay)...");
    let start = std::time::Instant::now();
    let result = consensus.propose(&block).await.unwrap();
    let elapsed = start.elapsed();
    
    match result {
        ConsensusResult::Committed(_) => {
            println!("✓ Block committed after {:?}!", elapsed);
        },
        _ => println!("Unexpected result"),
    }
}

/// Example: Test quorum-less consensus
pub async fn test_quorumless_consensus() {
    println!("\n=== Testing Quorum-less Consensus ===\n");
    
    let consensus = Arc::new(quorumless::QuorumlessConsensus::new(0, 5.0));
    
    // Set different weights for nodes
    consensus.set_node_weight(0, 2.0); // Node 0 has weight 2
    consensus.set_node_weight(1, 2.0); // Node 1 has weight 2
    consensus.set_node_weight(2, 1.5); // Node 2 has weight 1.5
    
    let block = Block {
        index: 1,
        timestamp: chrono::Utc::now().timestamp(),
        data: vec![MarketData {
            asset: "BTC".to_string(),
            price: 50000.0,
            source: "Test".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }],
        previous_hash: "0000_genesis".to_string(),
        hash: "test_hash".to_string(),
        nonce: 0,
    };
    
    println!("Proposing block with Quorum-less consensus (threshold: 5.0)...");
    println!("Node weights: 0=2.0, 1=2.0, 2=1.5");
    let result = consensus.propose(&block).await.unwrap();
    
    match result {
        ConsensusResult::Committed(_) => println!("✓ Block committed!"),
        ConsensusResult::Pending => println!("⏳ Block pending (need more votes)..."),
        ConsensusResult::Rejected(reason) => println!("✗ Block rejected: {}", reason),
    }
}
