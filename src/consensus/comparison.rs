//! Consensus algorithm comparison and benchmarking

use crate::consensus::{ConsensusRequirements, ConsensusResult};
use crate::etl::Block;
use async_trait::async_trait;
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

#[async_trait]
pub trait ConsensusStrategy: Send + Sync {
    async fn execute(&self, block: &Block) -> Result<Option<Block>, Box<dyn Error>>;
    fn name(&self) -> &str;
    fn requirements(&self) -> ConsensusRequirements;
    fn is_committed(&self, block_index: u64) -> bool;
}

pub struct NoConsensusStrategy {
    committed: Arc<parking_lot::RwLock<std::collections::HashSet<u64>>>,
}

impl NoConsensusStrategy {
    pub fn new() -> Self {
        Self {
            committed: Arc::new(parking_lot::RwLock::new(std::collections::HashSet::new())),
        }
    }
}

#[async_trait]
impl ConsensusStrategy for NoConsensusStrategy {
    async fn execute(&self, block: &Block) -> Result<Option<Block>, Box<dyn Error>> {
        let mut committed = self.committed.write();
        committed.insert(block.index);
        Ok(Some(block.clone()))
    }

    fn name(&self) -> &str {
        "No-Consensus (Single Node)"
    }

    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: false,
            min_nodes: Some(1),
            description: "No consensus required - single node confirmation".to_string(),
        }
    }

    fn is_committed(&self, block_index: u64) -> bool {
        let committed = self.committed.read();
        committed.contains(&block_index)
    }
}

pub struct SimpleMajorityStrategy {
    node_id: usize,
    total_nodes: usize,
    votes:
        Arc<parking_lot::RwLock<std::collections::HashMap<u64, std::collections::HashSet<usize>>>>,
    committed: Arc<parking_lot::RwLock<std::collections::HashSet<u64>>>,
}

impl SimpleMajorityStrategy {
    pub fn new(node_id: usize, total_nodes: usize) -> Self {
        Self {
            node_id,
            total_nodes,
            votes: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
            committed: Arc::new(parking_lot::RwLock::new(std::collections::HashSet::new())),
        }
    }

    fn majority_size(&self) -> usize {
        (self.total_nodes / 2) + 1
    }
}

#[async_trait]
impl ConsensusStrategy for SimpleMajorityStrategy {
    async fn execute(&self, block: &Block) -> Result<Option<Block>, Box<dyn Error>> {
        // Simulate collecting votes from other nodes
        let mut votes = self.votes.write();
        let block_votes = votes
            .entry(block.index)
            .or_insert_with(std::collections::HashSet::new);

        // Add our own vote
        block_votes.insert(self.node_id);

        // Simulate other nodes voting (for demo purposes)
        // In real implementation, this would come from network messages
        for i in 0..self.total_nodes {
            if i != self.node_id {
                block_votes.insert(i);
            }
        }

        let vote_count = block_votes.len();
        let majority = self.majority_size();

        if vote_count >= majority {
            let mut committed = self.committed.write();
            committed.insert(block.index);
            Ok(Some(block.clone()))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "Simple Majority (Non-BFT)"
    }

    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: true,
            min_nodes: Some(self.majority_size()),
            description: format!(
                "Simple majority voting: requires {}/{} votes (non-Byzantine)",
                self.majority_size(),
                self.total_nodes
            ),
        }
    }

    fn is_committed(&self, block_index: u64) -> bool {
        let committed = self.committed.read();
        committed.contains(&block_index)
    }
}

pub struct SimplifiedPoWStrategy {
    difficulty: usize,
    committed: Arc<parking_lot::RwLock<std::collections::HashSet<u64>>>,
}

impl SimplifiedPoWStrategy {
    pub fn new(difficulty: usize) -> Self {
        Self {
            difficulty,
            committed: Arc::new(parking_lot::RwLock::new(std::collections::HashSet::new())),
        }
    }

    fn mine_block(&self, block: &mut Block) {
        let target_prefix = "0".repeat(self.difficulty);

        loop {
            block.calculate_hash_with_nonce();
            if block.hash.starts_with(&target_prefix) {
                break;
            }
            block.nonce += 1;

            if block.nonce > 100000 {
                break;
            }
        }
    }
}

#[async_trait]
impl ConsensusStrategy for SimplifiedPoWStrategy {
    async fn execute(&self, block: &Block) -> Result<Option<Block>, Box<dyn Error>> {
        let mut block_to_mine = block.clone();

        self.mine_block(&mut block_to_mine);

        let target_prefix = "0".repeat(self.difficulty);
        if block_to_mine.hash.starts_with(&target_prefix) {
            let mut committed = self.committed.write();
            committed.insert(block_to_mine.index);
            Ok(Some(block_to_mine))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        "Simplified PoW"
    }

    fn requirements(&self) -> ConsensusRequirements {
        ConsensusRequirements {
            requires_majority: false,
            min_nodes: None,
            description: format!(
                "Proof-of-Work: requires hash with {} leading zeros",
                self.difficulty
            ),
        }
    }

    fn is_committed(&self, block_index: u64) -> bool {
        let committed = self.committed.read();
        committed.contains(&block_index)
    }
}

pub struct ConsensusAlgorithmAdapter {
    algorithm: Arc<dyn crate::consensus::ConsensusAlgorithm>,
}

impl ConsensusAlgorithmAdapter {
    pub fn new(algorithm: Arc<dyn crate::consensus::ConsensusAlgorithm>) -> Self {
        Self { algorithm }
    }
}

#[async_trait]
impl ConsensusStrategy for ConsensusAlgorithmAdapter {
    async fn execute(&self, block: &Block) -> Result<Option<Block>, Box<dyn Error>> {
        match self.algorithm.propose(block).await? {
            ConsensusResult::Committed(committed_block) => Ok(Some(committed_block)),
            ConsensusResult::Pending => Ok(None),
            ConsensusResult::Rejected(_) => Ok(None),
        }
    }

    fn name(&self) -> &str {
        self.algorithm.name()
    }

    fn requirements(&self) -> ConsensusRequirements {
        self.algorithm.requirements()
    }

    fn is_committed(&self, block_index: u64) -> bool {
        self.algorithm.is_committed(block_index)
    }
}

#[derive(Debug, Clone)]
pub struct ConsensusComparisonResult {
    pub strategy_name: String,
    pub block_index: u64,
    pub committed: bool,
    pub execution_time_ms: u64,
    pub requirements: ConsensusRequirements,
    pub error_occurred: bool,
    pub data_integrity: bool,
}

#[derive(Debug, Clone)]
pub struct ConsensusMetrics {
    pub strategy_name: String,
    pub total_blocks: usize,
    pub committed_blocks: usize,
    pub failed_blocks: usize,
    pub error_blocks: usize,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub avg_latency_ms: f64,
    pub throughput_blocks_per_sec: f64,
    pub error_rate: f64,
    pub commit_rate: f64,
    pub data_integrity_maintained: bool,
}

pub async fn compare_consensus_strategies(
    block: &Block,
    strategies: Vec<Arc<dyn ConsensusStrategy>>,
) -> Vec<ConsensusComparisonResult> {
    let mut results = Vec::new();

    for strategy in strategies {
        let start = Instant::now();
        let result = strategy.execute(block).await;
        let elapsed = start.elapsed().as_millis() as u64;

        let (committed, error_occurred, data_integrity) = match result {
            Ok(Some(_)) => (true, false, true),
            Ok(None) => (false, false, true),
            Err(_) => (false, true, false),
        };

        results.push(ConsensusComparisonResult {
            strategy_name: strategy.name().to_string(),
            block_index: block.index,
            committed,
            execution_time_ms: elapsed,
            requirements: strategy.requirements(),
            error_occurred,
            data_integrity,
        });
    }

    results
}

/// Run consensus benchmark with multiple blocks
///
/// This function runs a consensus strategy on multiple blocks to measure:
/// - Throughput (blocks per second)
/// - Latency statistics (min, max, avg)
/// - Error rate
/// - Stability in multi-block scenarios
/// - Data integrity on errors
pub async fn benchmark_consensus_strategy(
    strategy: Arc<dyn ConsensusStrategy>,
    blocks: &[Block],
) -> ConsensusMetrics {
    let mut latencies = Vec::new();
    let mut committed_count = 0;
    let mut failed_count = 0;
    let mut error_count = 0;
    let mut data_integrity_maintained = true;
    let total_start = Instant::now();

    for block in blocks {
        let start = Instant::now();
        let result = strategy.execute(block).await;
        let elapsed = start.elapsed().as_millis() as u64;
        latencies.push(elapsed);

        match result {
            Ok(Some(_)) => {
                committed_count += 1;
            }
            Ok(None) => {
                failed_count += 1;
            }
            Err(_) => {
                error_count += 1;
                if strategy.is_committed(block.index) {
                    data_integrity_maintained = false;
                }
            }
        }
    }

    let total_time = total_start.elapsed().as_secs_f64();
    let throughput = if total_time > 0.0 {
        blocks.len() as f64 / total_time
    } else {
        0.0
    };

    let min_latency = latencies.iter().min().copied().unwrap_or(0);
    let max_latency = latencies.iter().max().copied().unwrap_or(0);
    let avg_latency = if !latencies.is_empty() {
        latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
    } else {
        0.0
    };

    let error_rate = if !blocks.is_empty() {
        (error_count as f64 / blocks.len() as f64) * 100.0
    } else {
        0.0
    };

    let commit_rate = if !blocks.is_empty() {
        (committed_count as f64 / blocks.len() as f64) * 100.0
    } else {
        0.0
    };

    ConsensusMetrics {
        strategy_name: strategy.name().to_string(),
        total_blocks: blocks.len(),
        committed_blocks: committed_count,
        failed_blocks: failed_count,
        error_blocks: error_count,
        min_latency_ms: min_latency,
        max_latency_ms: max_latency,
        avg_latency_ms: avg_latency,
        throughput_blocks_per_sec: throughput,
        error_rate,
        commit_rate,
        data_integrity_maintained,
    }
}

pub async fn compare_consensus_with_metrics(
    blocks: &[Block],
    strategies: Vec<Arc<dyn ConsensusStrategy>>,
) -> Vec<ConsensusMetrics> {
    let mut metrics = Vec::new();

    for strategy in strategies {
        let metric = benchmark_consensus_strategy(strategy, blocks).await;
        metrics.push(metric);
    }

    metrics
}

/// Print comparison results in a formatted table
pub fn print_comparison_results(results: &[ConsensusComparisonResult]) {
    println!("\n{}", "=".repeat(120));
    println!("  Consensus Algorithm Comparison Results");
    println!("{}", "=".repeat(120));
    println!();
    println!(
        "{:<30} | {:<12} | {:<10} | {:<10} | {:<15} | {:<20}",
        "Strategy", "Committed", "Time (ms)", "Error", "Data Integrity", "Description"
    );
    println!("{}", "-".repeat(120));

    for result in results {
        println!(
            "{:<30} | {:<12} | {:<10} | {:<10} | {:<15} | {}",
            result.strategy_name,
            if result.committed { "Yes" } else { "No" },
            result.execution_time_ms,
            if result.error_occurred { "Yes" } else { "No" },
            if result.data_integrity { "Yes" } else { "No" },
            result.requirements.description
        );
    }

    println!("{}", "=".repeat(120));
    println!();
}

pub fn print_metrics_comparison(metrics: &[ConsensusMetrics]) {
    println!("\n{}", "=".repeat(140));
    println!("  Consensus Algorithm Detailed Metrics Comparison");
    println!("{}", "=".repeat(140));
    println!();
    println!(
        "{:<25} | {:<8} | {:<8} | {:<10} | {:<10} | {:<10} | {:<10} | {:<10} | {:<8} | {:<8} | {}",
        "Strategy",
        "Total",
        "Commit",
        "Failed",
        "Error",
        "Min(ms)",
        "Max(ms)",
        "Avg(ms)",
        "Throughput",
        "Error%",
        "Integrity"
    );
    println!("{}", "-".repeat(140));

    for metric in metrics {
        println!("{:<25} | {:<8} | {:<8} | {:<10} | {:<10} | {:<10} | {:<10} | {:<10.2} | {:<8.2} | {:<8.2} | {}", 
            metric.strategy_name,
            metric.total_blocks,
            metric.committed_blocks,
            metric.failed_blocks,
            metric.error_blocks,
            metric.min_latency_ms,
            metric.max_latency_ms,
            metric.avg_latency_ms,
            metric.throughput_blocks_per_sec,
            metric.error_rate,
            if metric.data_integrity_maintained { "Yes" } else { "No" }
        );
    }

    println!("{}", "=".repeat(140));
    println!();

    println!("Summary:");
    println!();

    if let Some(fastest) = metrics.iter().max_by(|a, b| {
        a.throughput_blocks_per_sec
            .partial_cmp(&b.throughput_blocks_per_sec)
            .unwrap()
    }) {
        println!(
            "  Highest Throughput: {} ({:.2} blocks/sec)",
            fastest.strategy_name, fastest.throughput_blocks_per_sec
        );
    }

    if let Some(lowest) = metrics
        .iter()
        .min_by(|a, b| a.avg_latency_ms.partial_cmp(&b.avg_latency_ms).unwrap())
    {
        println!(
            "  Lowest Latency: {} (avg {:.2} ms)",
            lowest.strategy_name, lowest.avg_latency_ms
        );
    }

    if let Some(most_stable) = metrics
        .iter()
        .min_by(|a, b| a.error_rate.partial_cmp(&b.error_rate).unwrap())
    {
        println!(
            "  Most Stable: {} (error rate: {:.2}%)",
            most_stable.strategy_name, most_stable.error_rate
        );
    }

    let integrity_ok = metrics
        .iter()
        .filter(|m| m.data_integrity_maintained)
        .count();
    println!(
        "  Data Integrity: {}/{} strategies maintained integrity",
        integrity_ok,
        metrics.len()
    );

    println!();
}
