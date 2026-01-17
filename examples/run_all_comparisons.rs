// Run all three comparison experiments

use rust_market_ledger::consensus::comparison::*;
use rust_market_ledger::etl::{Block, MarketData};
use std::io;
use std::sync::Arc;
use std::time::Instant;

async fn run_no_consensus_example() {
    println!("\n{}", "=".repeat(80));
    println!("  Example A: No-Consensus (Single Node Direct Commit)");
    println!("{}", "=".repeat(80));
    println!();

    let block = Block {
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

    let strategy = Arc::new(NoConsensusStrategy::new());
    let start = Instant::now();
    match strategy.execute(&block).await {
        Ok(Some(committed_block)) => {
            let elapsed = start.elapsed();
            println!(
                "Block committed: latency={:.2}ms, index={}",
                elapsed.as_secs_f64() * 1000.0,
                committed_block.index
            );
        }
        _ => {}
    }
}

async fn run_simple_majority_example() {
    println!("\n{}", "=".repeat(80));
    println!("  Example B: Simple Majority Vote (Non-BFT)");
    println!("{}", "=".repeat(80));
    println!();

    let block = Block {
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

    let total_nodes = 4;
    let node_id = 0;
    let strategy = Arc::new(SimpleMajorityStrategy::new(node_id, total_nodes));
    let start = Instant::now();
    match strategy.execute(&block).await {
        Ok(Some(committed_block)) => {
            let elapsed = start.elapsed();
            println!(
                "Block committed: latency={:.2}ms, index={}",
                elapsed.as_secs_f64() * 1000.0,
                committed_block.index
            );
        }
        _ => {}
    }
}

async fn run_pbft_baseline_example() {
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

    let total_nodes = 4;
    let node_id = 0;
    let node_addresses = vec![
        "127.0.0.1:8000".to_string(),
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];

    use rust_market_ledger::consensus::algorithms::pbft::PBFTConsensus;
    use rust_market_ledger::consensus::algorithms::PBFTManager;
    use rust_market_ledger::consensus::comparison::{ConsensusAlgorithmAdapter, ConsensusStrategy};

    let pbft_manager = Arc::new(PBFTManager::new(
        node_id,
        total_nodes,
        node_addresses.clone(),
    ));
    let pbft_consensus = Arc::new(PBFTConsensus::new(
        pbft_manager.clone(),
        node_addresses.clone(),
        8000,
    ));

    let strategy: Arc<dyn ConsensusStrategy> =
        Arc::new(ConsensusAlgorithmAdapter::new(pbft_consensus));
    let start = Instant::now();
    match strategy.execute(&block).await {
        Ok(Some(committed_block)) => {
            let elapsed = start.elapsed();
            println!(
                "Block committed: latency={:.2}ms, index={}",
                elapsed.as_secs_f64() * 1000.0,
                committed_block.index
            );
        }
        _ => {}
    }
}

#[tokio::main]
async fn main() {
    println!("\n{}", "=".repeat(80));
    println!("  Consensus Algorithm Comparison Experiments");
    println!("{}", "=".repeat(80));
    println!();
    println!("Press Enter to continue...");
    let mut _buffer = String::new();
    let _ = io::stdin().read_line(&mut _buffer);

    run_no_consensus_example().await;

    println!("\nPress Enter to continue to Example B...");
    let mut _buffer = String::new();
    let _ = io::stdin().read_line(&mut _buffer);

    run_simple_majority_example().await;

    println!("\nPress Enter to continue to Example C...");
    let mut _buffer = String::new();
    let _ = io::stdin().read_line(&mut _buffer);

    run_pbft_baseline_example().await;

    println!("\n{}", "=".repeat(80));
    println!("  All Examples Completed!");
    println!("{}", "=".repeat(80));
    println!();
    println!("Summary:");
    println!("  Example A: No-Consensus = Zero latency, Zero safety");
    println!("  Example B: Simple Majority = Low latency, No BFT");
    println!("  Example C: PBFT = Higher latency, Full BFT");
    println!();
    println!("Key Takeaway: Consensus is a trade-off between latency, safety, and complexity.");
    println!();
}
