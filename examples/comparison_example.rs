// Example: Compare different consensus strategies

use rust_market_ledger::consensus::algorithms::*;
use rust_market_ledger::consensus::comparison::*;
use rust_market_ledger::etl::{Block, MarketData};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("\n{}", "=".repeat(100));
    println!("  Consensus Strategy Comparison Example");
    println!("{}", "=".repeat(100));
    println!();

    let test_block = Block {
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
        "Test Block: index={}, data={} @ ${}",
        test_block.index, test_block.data[0].asset, test_block.data[0].price
    );
    println!();

    let strategies: Vec<Arc<dyn ConsensusStrategy>> = vec![
        Arc::new(NoConsensusStrategy::new()),
        Arc::new(SimpleMajorityStrategy::new(0, 4)),
        Arc::new(SimplifiedPoWStrategy::new(2)),
        Arc::new(ConsensusAlgorithmAdapter::new(Arc::new(
            gossip::GossipConsensus::new(0, 3, 2),
        ))),
        Arc::new(ConsensusAlgorithmAdapter::new(Arc::new(
            eventual::EventualConsensus::new(0, 100, 1),
        ))),
    ];

    println!("Strategies:");
    for (i, strategy) in strategies.iter().enumerate() {
        println!(
            "  {}. {} - {}",
            i + 1,
            strategy.name(),
            strategy.requirements().description
        );
    }
    println!();

    let results = compare_consensus_strategies(&test_block, strategies).await;
    print_comparison_results(&results);

    let committed_count = results.iter().filter(|r| r.committed).count();
    println!("Analysis:");
    println!(
        "  {} out of {} strategies committed",
        committed_count,
        results.len()
    );

    if let Some(fastest) = results.iter().min_by_key(|r| r.execution_time_ms) {
        println!(
            "  Fastest: {} ({} ms)",
            fastest.strategy_name, fastest.execution_time_ms
        );
    }

    if let Some(slowest) = results.iter().max_by_key(|r| r.execution_time_ms) {
        println!(
            "  Slowest: {} ({} ms)",
            slowest.strategy_name, slowest.execution_time_ms
        );
    }
    println!();
}
