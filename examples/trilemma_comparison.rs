//! Blockchain Trilemma Comparison Experiment

use rust_market_ledger::consensus::algorithms::*;
use rust_market_ledger::consensus::comparison::*;
use rust_market_ledger::etl::{Block, MarketData};
use std::sync::Arc;
use std::time::Instant;

#[path = "shared/mod.rs"]
mod metrics;
use metrics::{
    calculate_average_metrics, calculate_metrics_std_dev, calculate_runtime_std_dev, MetricsStdDev,
};

struct TrilemmaScores {
    decentralization: f64,
    security: f64,
    scalability: f64,
}

struct StrategyResult {
    strategy_name: String,
    metrics: ConsensusMetrics,
    metrics_std_dev: MetricsStdDev,
    trilemma: TrilemmaScores,
    runtime_seconds: f64,
    runtime_std_dev: f64,
}

#[tokio::main]
async fn main() {
    let experiment_start = Instant::now();

    println!("\n{}", "=".repeat(100));
    println!("  Blockchain Trilemma Comparison Experiment");
    println!("  Comparing: PBFT, Gossip, Eventual, Quorum-less, Flexible Paxos");
    println!("{}", "=".repeat(100));
    println!();

    const BLOCKS_PER_ROUND: usize = 100;
    const ROUNDS: usize = 5;
    const TOTAL_NODES: usize = 4;
    const NODE_ID: usize = 0;

    const PBFT_QUORUM: usize = 3;
    const GOSSIP_FANOUT: usize = 2;
    const EVENTUAL_DELAY_MS: u64 = 500;
    const EVENTUAL_THRESHOLD: usize = 2;
    const QUORUMLESS_THRESHOLD: f64 = 5.0;
    const FLEXIBLE_PAXOS_Q1: usize = 2;
    const FLEXIBLE_PAXOS_Q2: usize = 3;

    println!("Experiment Configuration (FIXED for reproducibility):");
    println!("  Blocks per round: {}", BLOCKS_PER_ROUND);
    println!(
        "  Rounds per strategy: {} (for statistical significance)",
        ROUNDS
    );
    println!("  Total blocks per strategy: {}", BLOCKS_PER_ROUND * ROUNDS);
    println!("  Total nodes: {}", TOTAL_NODES);
    println!();
    println!("Consensus Algorithm Parameters:");
    println!(
        "  PBFT: quorum={} (2f+1, f=1, total={})",
        PBFT_QUORUM, TOTAL_NODES
    );
    println!("  Gossip: fanout={}", GOSSIP_FANOUT);
    println!(
        "  Eventual: delay={}ms, threshold={}",
        EVENTUAL_DELAY_MS, EVENTUAL_THRESHOLD
    );
    println!("  Quorum-less: threshold={}", QUORUMLESS_THRESHOLD);
    println!(
        "  Flexible Paxos: Q1={}, Q2={}",
        FLEXIBLE_PAXOS_Q1, FLEXIBLE_PAXOS_Q2
    );
    println!();
    println!("Data Source: Simulated ETL data (offline/mock)");
    println!("Network: Simulated (single-machine simulation)");
    println!("  Note: PBFT has network handler but runs in simulated mode");
    println!("  Other algorithms use simulated consensus logic");
    println!();

    let mut blocks: Vec<Block> = Vec::new();
    for i in 1..=BLOCKS_PER_ROUND {
        let previous_hash = if i == 1 {
            "0000_genesis".to_string()
        } else {
            blocks[(i - 2) as usize].hash.clone()
        };

        let mut block = Block {
            index: i as u64,
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

    let node_addresses = vec![
        "127.0.0.1:8000".to_string(),
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];

    let pbft_manager = Arc::new(PBFTManager::new(
        NODE_ID,
        TOTAL_NODES,
        node_addresses.clone(),
    ));
    let pbft_consensus = Arc::new(pbft::PBFTConsensus::new(
        pbft_manager.clone(),
        node_addresses.clone(),
        8000,
    ));

    let strategies: Vec<(String, Arc<dyn ConsensusStrategy>)> = vec![
        (
            "PBFT".to_string(),
            Arc::new(ConsensusAlgorithmAdapter::new(pbft_consensus)),
        ),
        (
            "Gossip".to_string(),
            Arc::new(ConsensusAlgorithmAdapter::new(Arc::new(
                gossip::GossipConsensus::new(NODE_ID, TOTAL_NODES, GOSSIP_FANOUT),
            ))),
        ),
        (
            "Eventual".to_string(),
            Arc::new(ConsensusAlgorithmAdapter::new(Arc::new(
                eventual::EventualConsensus::new(NODE_ID, EVENTUAL_DELAY_MS, EVENTUAL_THRESHOLD),
            ))),
        ),
        (
            "Quorum-less".to_string(),
            Arc::new(ConsensusAlgorithmAdapter::new(Arc::new(
                quorumless::QuorumlessConsensus::new(NODE_ID, QUORUMLESS_THRESHOLD),
            ))),
        ),
        (
            "Flexible Paxos".to_string(),
            Arc::new(ConsensusAlgorithmAdapter::new(Arc::new(
                flexible_paxos::FlexiblePaxos::new(
                    NODE_ID,
                    TOTAL_NODES,
                    FLEXIBLE_PAXOS_Q1,
                    FLEXIBLE_PAXOS_Q2,
                ),
            ))),
        ),
    ];

    println!("Strategies to test:");
    for (i, (name, _)) in strategies.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    println!();

    let mut all_results: Vec<StrategyResult> = Vec::new();

    for (strategy_name, strategy) in &strategies {
        println!("Testing {}...", strategy_name);

        let mut round_metrics: Vec<ConsensusMetrics> = Vec::new();
        let mut round_runtimes: Vec<f64> = Vec::new();

        for round in 1..=ROUNDS {
            print!("  Round {}/{}... ", round, ROUNDS);
            let round_start = Instant::now();
            let metrics = benchmark_consensus_strategy(strategy.clone(), &blocks).await;
            let round_elapsed = round_start.elapsed().as_secs_f64();
            round_metrics.push(metrics);
            round_runtimes.push(round_elapsed);
            println!("Done ({:.2}s)", round_elapsed);
        }

        let strategy_runtime = round_runtimes.iter().sum::<f64>() / round_runtimes.len() as f64;
        let avg_metrics = calculate_average_metrics(&round_metrics);
        let metrics_std_dev = calculate_metrics_std_dev(&round_metrics, &avg_metrics);
        let runtime_std_dev = calculate_runtime_std_dev(&round_runtimes);
        let trilemma = get_trilemma_scores(strategy_name);

        all_results.push(StrategyResult {
            strategy_name: strategy_name.clone(),
            metrics: avg_metrics,
            metrics_std_dev,
            trilemma,
            runtime_seconds: strategy_runtime,
            runtime_std_dev,
        });

        println!(
            "  {} completed in {:.2}s\n",
            strategy_name, strategy_runtime
        );
    }

    let total_runtime = experiment_start.elapsed();
    print_runtime_summary(&all_results, total_runtime, ROUNDS);
    print_trilemma_comparison_table(&all_results, ROUNDS);
    print_trilemma_analysis(&all_results);

    println!("{}", "=".repeat(100));
    println!(
        "Experiment completed in {:.2}s ({:.2} minutes)",
        total_runtime.as_secs_f64(),
        total_runtime.as_secs_f64() / 60.0
    );
    println!("{}", "=".repeat(100));
}

fn get_trilemma_scores(strategy_name: &str) -> TrilemmaScores {
    match strategy_name {
        "PBFT" => TrilemmaScores {
            decentralization: 3.0,
            security: 5.0,
            scalability: 2.0,
        },
        "Gossip" => TrilemmaScores {
            decentralization: 5.0,
            security: 2.0,
            scalability: 4.0,
        },
        "Eventual" => TrilemmaScores {
            decentralization: 4.0,
            security: 2.0,
            scalability: 4.0,
        },
        "Quorum-less" => TrilemmaScores {
            decentralization: 4.0,
            security: 3.0,
            scalability: 3.0,
        },
        "Flexible Paxos" => TrilemmaScores {
            decentralization: 3.0,
            security: 4.0,
            scalability: 3.0,
        },
        _ => TrilemmaScores {
            decentralization: 3.0,
            security: 3.0,
            scalability: 3.0,
        },
    }
}

fn print_runtime_summary(
    results: &[StrategyResult],
    total_runtime: std::time::Duration,
    rounds: usize,
) {
    println!("\n{}", "=".repeat(120));
    println!("  Runtime Summary (for Medium article credibility)");
    println!("{}", "=".repeat(120));
    println!();

    println!("Experiment Runtime Breakdown (Mean ± Std Dev):");
    println!(
        "{:<20} | {:>18} | {:>18} | {:>15}",
        "Strategy", "Runtime (s)", "Runtime (min)", "Blocks/sec"
    );
    println!("{}", "-".repeat(120));

    for result in results {
        let blocks_per_sec = if result.runtime_seconds > 0.0 {
            result.metrics.total_blocks as f64 / result.runtime_seconds
        } else {
            0.0
        };

        println!(
            "{:<20} | {:>8.2} ± {:>6.2} | {:>8.2} ± {:>6.2} | {:>15.2}",
            result.strategy_name,
            result.runtime_seconds,
            result.runtime_std_dev,
            result.runtime_seconds / 60.0,
            result.runtime_std_dev / 60.0,
            blocks_per_sec
        );
    }

    println!();
    println!("Total Experiment Runtime:");
    println!(
        "  {:.2} seconds ({:.2} minutes)",
        total_runtime.as_secs_f64(),
        total_runtime.as_secs_f64() / 60.0
    );
    println!();

    println!("System Information (for reproducibility):");
    println!("  OS: {}", std::env::consts::OS);
    println!("  Architecture: {}", std::env::consts::ARCH);
    if let Ok(rustc_version) = std::process::Command::new("rustc")
        .arg("--version")
        .output()
    {
        if let Ok(version) = String::from_utf8(rustc_version.stdout) {
            println!("  Rust Version: {}", version.trim());
        }
    }
    println!();

    println!("Experimental Scope and Limitations:");
    println!("  - Network: Simulated (single-machine)");
    println!("  - PBFT: Has network handler but runs in simulated mode");
    println!("  - Other algorithms: Use simulated consensus logic");
    println!("  - Data: Simulated ETL data (offline/mock)");
    println!("  - All strategies use the SAME blocks for fair comparison");
    println!(
        "  - Results are averaged over {} runs with std dev reported",
        rounds
    );
    println!();
}

fn print_trilemma_comparison_table(results: &[StrategyResult], rounds: usize) {
    println!("\n{}", "=".repeat(120));
    println!("  Comprehensive Trilemma Comparison Table");
    println!("{}", "=".repeat(120));
    println!();

    println!("Performance Metrics (Mean ± Std Dev, n={}):", rounds);
    println!(
        "{:<20} | {:>12} | {:>12} | {:>12} | {:>12} | {:>12}",
        "Strategy", "Latency (ms)", "Throughput", "Commit Rate", "Error Rate", "Integrity"
    );
    println!("{}", "-".repeat(120));

    for result in results {
        println!(
            "{:<20} | {:>6.2} ± {:>5.2} | {:>6.2} ± {:>5.2} | {:>6.2} ± {:>5.2} | {:>6.2} ± {:>5.2} | {:>12}",
            result.strategy_name,
            result.metrics.avg_latency_ms,
            result.metrics_std_dev.latency_std_dev,
            result.metrics.throughput_blocks_per_sec,
            result.metrics_std_dev.throughput_std_dev,
            result.metrics.commit_rate,
            result.metrics_std_dev.commit_rate_std_dev,
            result.metrics.error_rate,
            result.metrics_std_dev.error_rate_std_dev,
            if result.metrics.data_integrity_maintained { "Yes" } else { "No" }
        );
    }

    println!();
    println!("Extended Trilemma Metrics (arXiv:2505.03768 - 15 Metrics):");
    println!();
    println!("Degree of Decentralization (DoD) - 5 metrics:");
    println!(
        "{:<20} | {:>18} | {:>18} | {:>18} | {:>18} | {:>18}",
        "Strategy",
        "Block Proposal Rand",
        "Geographic Diversity",
        "Hash Power Dist",
        "Token Concentration",
        "Wealth Distribution"
    );
    println!("{}", "-".repeat(140));
    for result in results {
        println!(
            "{:<20} | {:>18} | {:>18} | {:>18} | {:>18} | {:>18}",
            result.strategy_name,
            result
                .metrics
                .block_proposal_randomness
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            result
                .metrics
                .geographical_diversity
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            result
                .metrics
                .hashing_power_distribution
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            result
                .metrics
                .token_concentration
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            result
                .metrics
                .wealth_distribution
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
        );
    }
    println!();
    println!("Scalability - 3 metrics:");
    println!(
        "{:<20} | {:>15} | {:>20} | {:>20}",
        "Strategy", "Availability (%)", "Confirmation Latency (ms)", "Max Throughput (TPS)"
    );
    println!("{}", "-".repeat(100));
    for result in results {
        println!(
            "{:<20} | {:>15.2} | {:>20.2} | {:>20.2}",
            result.strategy_name,
            result.metrics.availability,
            result.metrics.confirmation_latency_ms,
            result.metrics.max_throughput_tps,
        );
    }
    println!();
    println!("Security - 4 metrics:");
    println!(
        "{:<20} | {:>15} | {:>20} | {:>15} | {:>20}",
        "Strategy", "Cost of Attack", "Fault Tolerance", "Reliability (%)", "Stale Block Rate (%)"
    );
    println!("{}", "-".repeat(110));
    for result in results {
        println!(
            "{:<20} | {:>15} | {:>20.2} | {:>15.2} | {:>20.2}",
            result.strategy_name,
            result
                .metrics
                .cost_of_attack
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string()),
            result.metrics.fault_tolerance,
            result.metrics.reliability,
            result.metrics.stale_block_rate,
        );
    }
    println!();
    println!("Trilemma Scores (1-5 scale):");
    println!(
        "{:<20} | {:>15} | {:>15} | {:>15} | {:>15}",
        "Strategy", "Decentralization", "Security", "Scalability", "Total"
    );
    println!("{}", "-".repeat(120));

    for result in results {
        let total = result.trilemma.decentralization
            + result.trilemma.security
            + result.trilemma.scalability;
        println!(
            "{:<20} | {:>15.1} | {:>15.1} | {:>15.1} | {:>15.1}",
            result.strategy_name,
            result.trilemma.decentralization,
            result.trilemma.security,
            result.trilemma.scalability,
            total
        );
    }

    println!();
}

fn print_trilemma_analysis(results: &[StrategyResult]) {
    println!("{}", "=".repeat(120));
    println!("  Trilemma Analysis: Trade-offs and Sacrifices");
    println!("{}", "=".repeat(120));
    println!();

    for result in results {
        println!("{}:", result.strategy_name);

        let scores = &result.trilemma;
        let min_score = scores
            .decentralization
            .min(scores.security)
            .min(scores.scalability);
        let sacrifice = if scores.scalability == min_score {
            "Scalability"
        } else if scores.security == min_score {
            "Security"
        } else {
            "Decentralization"
        };

        println!("  Primary Sacrifice: {}", sacrifice);
        println!(
            "  Performance: latency={:.2}ms, throughput={:.2} blocks/sec",
            result.metrics.avg_latency_ms, result.metrics.throughput_blocks_per_sec
        );
        println!(
            "  Reliability: commit_rate={:.2}%, error_rate={:.2}%",
            result.metrics.commit_rate, result.metrics.error_rate
        );
        println!();
    }

    println!("Best Performers:");
    if let Some(best_latency) = results.iter().min_by(|a, b| {
        a.metrics
            .avg_latency_ms
            .partial_cmp(&b.metrics.avg_latency_ms)
            .unwrap()
    }) {
        println!(
            "  Lowest Latency: {} ({:.2} ms)",
            best_latency.strategy_name, best_latency.metrics.avg_latency_ms
        );
    }

    if let Some(best_throughput) = results.iter().max_by(|a, b| {
        a.metrics
            .throughput_blocks_per_sec
            .partial_cmp(&b.metrics.throughput_blocks_per_sec)
            .unwrap()
    }) {
        println!(
            "  Highest Throughput: {} ({:.2} blocks/sec)",
            best_throughput.strategy_name, best_throughput.metrics.throughput_blocks_per_sec
        );
    }

    if let Some(best_security) = results.iter().max_by(|a, b| {
        a.trilemma
            .security
            .partial_cmp(&b.trilemma.security)
            .unwrap()
    }) {
        println!(
            "  Highest Security: {} (score: {:.1})",
            best_security.strategy_name, best_security.trilemma.security
        );
    }

    if let Some(best_scalability) = results.iter().max_by(|a, b| {
        a.trilemma
            .scalability
            .partial_cmp(&b.trilemma.scalability)
            .unwrap()
    }) {
        println!(
            "  Highest Scalability: {} (score: {:.1})",
            best_scalability.strategy_name, best_scalability.trilemma.scalability
        );
    }

    println!();
}
