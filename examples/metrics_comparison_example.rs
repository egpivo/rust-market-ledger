// Example: Compare consensus algorithms with detailed metrics

use rust_market_ledger::consensus::algorithms::pbft::PBFTConsensus;
use rust_market_ledger::consensus::algorithms::PBFTManager;
use rust_market_ledger::consensus::comparison::*;
use rust_market_ledger::etl::{Block, MarketData};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("\n{}", "=".repeat(100));
    println!("  Comprehensive Consensus Metrics Comparison");
    println!("{}", "=".repeat(100));
    println!();

    let mut blocks: Vec<Block> = Vec::new();
    for i in 1..=10 {
        let previous_hash = if i == 1 {
            "0000_genesis".to_string()
        } else {
            blocks[(i - 2) as usize].hash.clone()
        };

        let mut block = Block {
            index: i,
            timestamp: chrono::Utc::now().timestamp() + i as i64,
            data: vec![MarketData {
                asset: "BTC".to_string(),
                price: 50000.0 + (i as f32 * 100.0),
                source: "CoinGecko".to_string(),
                timestamp: chrono::Utc::now().timestamp() + i as i64,
            }],
            previous_hash,
            hash: String::new(),
            nonce: 0,
        };
        block.calculate_hash_with_nonce();
        blocks.push(block);
    }

    println!("Generated {} test blocks", blocks.len());
    println!();

    let total_nodes = 4;
    let node_id = 0;
    let node_addresses = vec![
        "127.0.0.1:8000".to_string(),
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];

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

    let strategies: Vec<Arc<dyn ConsensusStrategy>> = vec![
        Arc::new(NoConsensusStrategy::new()),
        Arc::new(SimpleMajorityStrategy::new(node_id, total_nodes)),
        Arc::new(ConsensusAlgorithmAdapter::new(pbft_consensus)),
    ];

    println!("Strategies:");
    for (i, strategy) in strategies.iter().enumerate() {
        println!("  {}. {}", i + 1, strategy.name());
    }
    println!();

    println!(
        "Executing benchmark ({} blocks per strategy)...",
        blocks.len()
    );
    println!();

    let metrics = compare_consensus_with_metrics(&blocks, strategies).await;

    print_metrics_comparison(&metrics);

    println!("Detailed Analysis:");
    println!();

    for metric in &metrics {
        println!("{}:", metric.strategy_name);
        println!(
            "  Throughput: {:.2} blocks/sec",
            metric.throughput_blocks_per_sec
        );
        println!(
            "  Latency: min={}ms, max={}ms, avg={:.2}ms",
            metric.min_latency_ms, metric.max_latency_ms, metric.avg_latency_ms
        );
        println!(
            "  Commit Rate: {:.2}% ({}/{})",
            metric.commit_rate, metric.committed_blocks, metric.total_blocks
        );
        println!(
            "  Error Rate: {:.2}% ({}/{})",
            metric.error_rate, metric.error_blocks, metric.total_blocks
        );
        println!(
            "  Data Integrity: {}",
            if metric.data_integrity_maintained {
                "Maintained"
            } else {
                "Compromised"
            }
        );
        println!();
    }

    println!("Comparison Insights:");
    println!();

    let no_consensus = metrics
        .iter()
        .find(|m| m.strategy_name == "No-Consensus (Single Node)");
    let pbft = metrics.iter().find(|m| m.strategy_name == "PBFT");

    if let (Some(no_cons), Some(pbft_metric)) = (no_consensus, pbft) {
        let throughput_diff =
            no_cons.throughput_blocks_per_sec / pbft_metric.throughput_blocks_per_sec.max(0.01);
        println!("Throughput:");
        println!(
            "  No-Consensus: {:.2} blocks/sec",
            no_cons.throughput_blocks_per_sec
        );
        println!(
            "  PBFT: {:.2} blocks/sec",
            pbft_metric.throughput_blocks_per_sec
        );
        println!("  No-Consensus is {:.2}x faster", throughput_diff);
        println!();

        let latency_diff = pbft_metric.avg_latency_ms / no_cons.avg_latency_ms.max(0.01);
        println!("Latency:");
        println!("  No-Consensus: {:.2} ms (avg)", no_cons.avg_latency_ms);
        println!("  PBFT: {:.2} ms (avg)", pbft_metric.avg_latency_ms);
        println!("  PBFT latency is {:.2}x of No-Consensus", latency_diff);
        println!();
    }

    println!("Stability (error rate):");
    for metric in &metrics {
        println!("  {}: {:.2}%", metric.strategy_name, metric.error_rate);
    }
    println!();

    println!("Data Integrity:");
    for metric in &metrics {
        let status = if metric.data_integrity_maintained {
            "Maintained"
        } else {
            "Compromised"
        };
        println!("  {}: {}", metric.strategy_name, status);
    }
    println!();

    println!("{}", "=".repeat(100));
    println!();
}
