// Example C: PBFT (Baseline for Comparison)

use rust_market_ledger::consensus::comparison::{ConsensusAlgorithmAdapter, ConsensusStrategy};
use rust_market_ledger::consensus::algorithms::pbft::PBFTConsensus;
use rust_market_ledger::consensus::algorithms::PBFTManager;
use rust_market_ledger::etl::{Block, MarketData};
use std::sync::Arc;
use std::time::Instant;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("\n{}", "=".repeat(80));
    println!("  Example C: PBFT (Baseline for Comparison)");
    println!("{}", "=".repeat(80));
    println!();
    
    let mut block = Block {
        index: 1,
        timestamp: chrono::Utc::now().timestamp(),
        data: vec![MarketData {
            asset: "BTC".to_string(),
            price: 50000.0,
            source: "CoinGecko".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }],
        previous_hash: "0000_genesis".to_string(),
        hash: String::new(),
        nonce: 0,
    };
    block.calculate_hash_with_nonce();
    
    println!("Block created: index={}, data={} @ ${}, hash={}...", 
        block.index, block.data[0].asset, block.data[0].price,
        &block.hash[0..8.min(block.hash.len())]);
    println!();
    
    let total_nodes = 4;
    let node_id = 0;
    let node_addresses = vec![
        "127.0.0.1:8000".to_string(),
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];
    
    let f = (total_nodes - 1) / 3;
    let quorum = (2 * f) + 1;
    
    println!("Strategy: PBFT");
    println!("Nodes: {}, Max faulty (f): {}, Quorum: {}/{} (2f+1)", 
        total_nodes, f, quorum, total_nodes);
    println!();
    
    let pbft_manager = Arc::new(PBFTManager::new(node_id, total_nodes, node_addresses.clone()));
    let pbft_consensus = Arc::new(PBFTConsensus::new(
        pbft_manager.clone(),
        node_addresses.clone(),
        8000,
    ));
    
    let strategy: Arc<dyn ConsensusStrategy> = Arc::new(ConsensusAlgorithmAdapter::new(pbft_consensus));
    
    let start = Instant::now();
    
    println!("Phase 1: Pre-Prepare");
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    println!("Phase 2: Prepare (quorum: {}/{})", quorum, total_nodes);
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    println!("Phase 3: Commit (quorum: {}/{})", quorum, total_nodes);
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    match strategy.execute(&block).await {
        Ok(Some(committed_block)) => {
            let elapsed = start.elapsed();
            println!();
            println!("Block committed: latency={:.2}ms, index={}", 
                elapsed.as_secs_f64() * 1000.0, committed_block.index);
            println!();
            println!("PBFT Advantages:");
            println!("  - Byzantine fault tolerance (tolerates f malicious nodes)");
            println!("  - Safety guarantee even with malicious nodes");
            println!("  - Three-phase validation");
            println!("  - Strong consistency");
            println!();
            println!("PBFT Costs:");
            println!("  - Higher latency (three-phase communication)");
            println!("  - Communication overhead");
            println!("  - Requires at least 3f+1 nodes");
            println!();
            println!("Comparison:");
            println!("  vs No-Consensus: PBFT has latency but has safety");
            println!("  vs Simple Majority: PBFT tolerates Byzantine faults");
            println!();
        }
        Ok(None) => println!("Block not committed (quorum not reached)"),
        Err(e) => println!("Error: {}", e),
    }
    
    println!("{}", "=".repeat(80));
    println!();
}
