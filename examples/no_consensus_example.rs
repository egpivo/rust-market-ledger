// Example A: No-Consensus (Single Node Direct Commit)

use rust_market_ledger::consensus::comparison::*;
use rust_market_ledger::etl::{Block, MarketData};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() {
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

    println!(
        "Block created: index={}, data={} @ ${}",
        block.index, block.data[0].asset, block.data[0].price
    );
    println!();

    let strategy = Arc::new(NoConsensusStrategy::new());
    println!("Strategy: {}", strategy.name());
    println!();

    let start = Instant::now();
    match strategy.execute(&block).await {
        Ok(Some(committed_block)) => {
            let elapsed = start.elapsed();
            println!(
                "Block committed: latency={:.2}ms, index={}",
                elapsed.as_secs_f64() * 1000.0,
                committed_block.index
            );
            println!();
            println!("Advantages:");
            println!("  - Zero latency, immediate commit");
            println!("  - No network overhead");
            println!("  - Simple");
            println!();
            println!("Disadvantages:");
            println!("  - No safety guarantees");
            println!("  - Cannot tolerate failures");
            println!("  - Not suitable for distributed systems");
            println!();
            println!("Comparison with PBFT:");
            println!("  - PBFT has latency but provides safety");
            println!("  - No-Consensus has zero latency but no safety");
            println!();
        }
        Ok(None) => println!("Block not committed"),
        Err(e) => println!("Error: {}", e),
    }

    println!("{}", "=".repeat(80));
    println!();
}
