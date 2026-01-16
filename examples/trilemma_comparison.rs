//! Blockchain Trilemma Comparison Experiment
//! 
//! This example demonstrates a comprehensive comparison of 5 consensus algorithms
//! in the context of the blockchain trilemma (Decentralization / Security / Scalability).

use rust_market_ledger::consensus::comparison::*;
use rust_market_ledger::consensus::algorithms::*;
use rust_market_ledger::etl::{Block, MarketData};
use std::sync::Arc;
use std::collections::HashMap;

/// Trilemma scores for each consensus algorithm
struct TrilemmaScores {
    decentralization: f64, // 1-5
    security: f64,          // 1-5
    scalability: f64,       // 1-5
}

/// Complete experiment result for a strategy
struct StrategyResult {
    strategy_name: String,
    metrics: ConsensusMetrics,
    trilemma: TrilemmaScores,
}

/// Run comprehensive trilemma comparison experiment
#[tokio::main]
async fn main() {
    println!("\n{}", "=".repeat(100));
    println!("  Blockchain Trilemma Comparison Experiment");
    println!("  Comparing: PBFT, Gossip, Eventual, Quorum-less, Flexible Paxos");
    println!("{}", "=".repeat(100));
    println!();
    
    // Experiment parameters
    const BLOCKS_PER_ROUND: usize = 100;
    const ROUNDS: usize = 5;
    
    println!("Experiment Configuration:");
    println!("  Blocks per round: {}", BLOCKS_PER_ROUND);
    println!("  Rounds per strategy: {}", ROUNDS);
    println!("  Total blocks per strategy: {}", BLOCKS_PER_ROUND * ROUNDS);
    println!();
    
    // Generate test blocks (same for all strategies)
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
    
    // Initialize consensus strategies
    let total_nodes = 4;
    let node_id = 0;
    let node_addresses = vec![
        "127.0.0.1:8000".to_string(),
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];
    
    // Create all strategies
    let pbft_manager = Arc::new(PBFTManager::new(node_id, total_nodes, node_addresses.clone()));
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
            Arc::new(ConsensusAlgorithmAdapter::new(
                Arc::new(gossip::GossipConsensus::new(node_id, total_nodes, 2))
            )),
        ),
        (
            "Eventual".to_string(),
            Arc::new(ConsensusAlgorithmAdapter::new(
                Arc::new(eventual::EventualConsensus::new(node_id, 500, 2))
            )),
        ),
        (
            "Quorum-less".to_string(),
            Arc::new(ConsensusAlgorithmAdapter::new(
                Arc::new(quorumless::QuorumlessConsensus::new(node_id, 5.0))
            )),
        ),
        (
            "Flexible Paxos".to_string(),
            Arc::new(ConsensusAlgorithmAdapter::new(
                Arc::new(flexible_paxos::FlexiblePaxos::new(node_id, total_nodes, 2, 3))
            )),
        ),
    ];
    
    println!("Strategies to test:");
    for (i, (name, _)) in strategies.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    println!();
    
    // Run experiments for each strategy
    let mut all_results: Vec<StrategyResult> = Vec::new();
    
    for (strategy_name, strategy) in &strategies {
        println!("Testing {}...", strategy_name);
        
        // Run multiple rounds and collect metrics
        let mut round_metrics: Vec<ConsensusMetrics> = Vec::new();
        
        for round in 1..=ROUNDS {
            print!("  Round {}/{}... ", round, ROUNDS);
            let metrics = benchmark_consensus_strategy(strategy.clone(), &blocks).await;
            round_metrics.push(metrics);
            println!("Done");
        }
        
        // Calculate average metrics across rounds
        let avg_metrics = calculate_average_metrics(&round_metrics);
        
        // Assign trilemma scores based on algorithm characteristics
        let trilemma = get_trilemma_scores(strategy_name);
        
        all_results.push(StrategyResult {
            strategy_name: strategy_name.clone(),
            metrics: avg_metrics,
            trilemma,
        });
        
        println!("  {} completed\n", strategy_name);
    }
    
    // Print comprehensive comparison table
    print_trilemma_comparison_table(&all_results);
    
    // Print detailed analysis
    print_trilemma_analysis(&all_results);
    
    println!("{}", "=".repeat(100));
    println!("Experiment completed!");
    println!("{}", "=".repeat(100));
}

/// Calculate average metrics across multiple rounds
fn calculate_average_metrics(round_metrics: &[ConsensusMetrics]) -> ConsensusMetrics {
    if round_metrics.is_empty() {
        return ConsensusMetrics {
            strategy_name: String::new(),
            total_blocks: 0,
            committed_blocks: 0,
            failed_blocks: 0,
            error_blocks: 0,
            min_latency_ms: 0,
            max_latency_ms: 0,
            avg_latency_ms: 0.0,
            throughput_blocks_per_sec: 0.0,
            error_rate: 0.0,
            commit_rate: 0.0,
            data_integrity_maintained: true,
        };
    }
    
    let count = round_metrics.len() as f64;
    let strategy_name = round_metrics[0].strategy_name.clone();
    
    ConsensusMetrics {
        strategy_name,
        total_blocks: round_metrics[0].total_blocks,
        committed_blocks: (round_metrics.iter().map(|m| m.committed_blocks).sum::<usize>() as f64 / count) as usize,
        failed_blocks: (round_metrics.iter().map(|m| m.failed_blocks).sum::<usize>() as f64 / count) as usize,
        error_blocks: (round_metrics.iter().map(|m| m.error_blocks).sum::<usize>() as f64 / count) as usize,
        min_latency_ms: round_metrics.iter().map(|m| m.min_latency_ms).min().unwrap_or(0),
        max_latency_ms: round_metrics.iter().map(|m| m.max_latency_ms).max().unwrap_or(0),
        avg_latency_ms: round_metrics.iter().map(|m| m.avg_latency_ms).sum::<f64>() / count,
        throughput_blocks_per_sec: round_metrics.iter().map(|m| m.throughput_blocks_per_sec).sum::<f64>() / count,
        error_rate: round_metrics.iter().map(|m| m.error_rate).sum::<f64>() / count,
        commit_rate: round_metrics.iter().map(|m| m.commit_rate).sum::<f64>() / count,
        data_integrity_maintained: round_metrics.iter().all(|m| m.data_integrity_maintained),
    }
}

/// Get trilemma scores for each consensus algorithm
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

/// Print comprehensive trilemma comparison table
fn print_trilemma_comparison_table(results: &[StrategyResult]) {
    println!("\n{}", "=".repeat(120));
    println!("  Comprehensive Trilemma Comparison Table");
    println!("{}", "=".repeat(120));
    println!();
    
    // Performance metrics table
    println!("Performance Metrics:");
    println!("{:<20} | {:>12} | {:>12} | {:>12} | {:>12} | {:>12}", 
        "Strategy", "Latency (ms)", "Throughput", "Commit Rate", "Error Rate", "Integrity");
    println!("{}", "-".repeat(120));
    
    for result in results {
        println!("{:<20} | {:>12.2} | {:>12.2} | {:>12.2}% | {:>12.2}% | {:>12}", 
            result.strategy_name,
            result.metrics.avg_latency_ms,
            result.metrics.throughput_blocks_per_sec,
            result.metrics.commit_rate,
            result.metrics.error_rate,
            if result.metrics.data_integrity_maintained { "Yes" } else { "No" }
        );
    }
    
    println!();
    
    // Trilemma scores table
    println!("Trilemma Scores (1-5 scale):");
    println!("{:<20} | {:>15} | {:>15} | {:>15} | {:>15}", 
        "Strategy", "Decentralization", "Security", "Scalability", "Total");
    println!("{}", "-".repeat(120));
    
    for result in results {
        let total = result.trilemma.decentralization + 
                    result.trilemma.security + 
                    result.trilemma.scalability;
        println!("{:<20} | {:>15.1} | {:>15.1} | {:>15.1} | {:>15.1}", 
            result.strategy_name,
            result.trilemma.decentralization,
            result.trilemma.security,
            result.trilemma.scalability,
            total
        );
    }
    
    println!();
}

/// Print detailed trilemma analysis
fn print_trilemma_analysis(results: &[StrategyResult]) {
    println!("{}", "=".repeat(120));
    println!("  Trilemma Analysis: Trade-offs and Sacrifices");
    println!("{}", "=".repeat(120));
    println!();
    
    for result in results {
        println!("{}:", result.strategy_name);
        
        // Identify primary sacrifice
        let scores = &result.trilemma;
        let min_score = scores.decentralization.min(scores.security).min(scores.scalability);
        let sacrifice = if scores.scalability == min_score {
            "Scalability"
        } else if scores.security == min_score {
            "Security"
        } else {
            "Decentralization"
        };
        
        println!("  Primary Sacrifice: {}", sacrifice);
        println!("  Performance: latency={:.2}ms, throughput={:.2} blocks/sec", 
            result.metrics.avg_latency_ms, result.metrics.throughput_blocks_per_sec);
        println!("  Reliability: commit_rate={:.2}%, error_rate={:.2}%", 
            result.metrics.commit_rate, result.metrics.error_rate);
        println!();
    }
    
    // Find best in each category
    println!("Best Performers:");
    if let Some(best_latency) = results.iter().min_by(|a, b| {
        a.metrics.avg_latency_ms.partial_cmp(&b.metrics.avg_latency_ms).unwrap()
    }) {
        println!("  Lowest Latency: {} ({:.2} ms)", 
            best_latency.strategy_name, best_latency.metrics.avg_latency_ms);
    }
    
    if let Some(best_throughput) = results.iter().max_by(|a, b| {
        a.metrics.throughput_blocks_per_sec.partial_cmp(&b.metrics.throughput_blocks_per_sec).unwrap()
    }) {
        println!("  Highest Throughput: {} ({:.2} blocks/sec)", 
            best_throughput.strategy_name, best_throughput.metrics.throughput_blocks_per_sec);
    }
    
    if let Some(best_security) = results.iter().max_by(|a, b| {
        a.trilemma.security.partial_cmp(&b.trilemma.security).unwrap()
    }) {
        println!("  Highest Security: {} (score: {:.1})", 
            best_security.strategy_name, best_security.trilemma.security);
    }
    
    if let Some(best_scalability) = results.iter().max_by(|a, b| {
        a.trilemma.scalability.partial_cmp(&b.trilemma.scalability).unwrap()
    }) {
        println!("  Highest Scalability: {} (score: {:.1})", 
            best_scalability.strategy_name, best_scalability.trilemma.scalability);
    }
    
    println!();
}
