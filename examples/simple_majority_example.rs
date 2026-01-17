// Example B: Simple Majority Vote (Non-BFT)

use rust_market_ledger::consensus::comparison::*;
use rust_market_ledger::etl::{Block, MarketData};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() {
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

    println!(
        "Block created: index={}, data={} @ ${}",
        block.index, block.data[0].asset, block.data[0].price
    );
    println!();

    let total_nodes = 4;
    let node_id = 0;
    let strategy = Arc::new(SimpleMajorityStrategy::new(node_id, total_nodes));
    let majority = (total_nodes / 2) + 1;

    println!("Strategy: {}", strategy.name());
    println!(
        "Nodes: {}, Majority: {}/{}",
        total_nodes, majority, total_nodes
    );
    println!();

    let start = Instant::now();
    match strategy.execute(&block).await {
        Ok(Some(committed_block)) => {
            let elapsed = start.elapsed();
            println!(
                "Block committed: latency={:.2}ms, index={}, votes={}/{}",
                elapsed.as_secs_f64() * 1000.0,
                committed_block.index,
                majority,
                total_nodes
            );
            println!();
            println!("Advantages:");
            println!("  - Simpler than PBFT");
            println!("  - Lower latency");
            println!("  - Suitable for non-Byzantine environments");
            println!();
            println!("Disadvantages:");
            println!("  - Cannot tolerate Byzantine faults");
            println!("  - Malicious nodes can break consensus");
            println!();
            println!("Comparison with PBFT:");
            println!("  - PBFT tolerates f Byzantine nodes (3f+1 total)");
            println!("  - PBFT has three-phase validation");
            println!("  - PBFT guarantees safety with malicious nodes");
            println!();
        }
        Ok(None) => println!("Block not committed (need more votes)"),
        Err(e) => println!("Error: {}", e),
    }

    println!("{}", "=".repeat(80));
    println!();
}
