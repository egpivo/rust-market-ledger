//! Shared metrics utilities for experiment examples

use rust_market_ledger::consensus::comparison::ConsensusMetrics;

pub struct MetricsStdDev {
    pub latency_std_dev: f64,
    pub throughput_std_dev: f64,
    pub commit_rate_std_dev: f64,
    pub error_rate_std_dev: f64,
}

pub fn calculate_runtime_std_dev(runtimes: &[f64]) -> f64 {
    if runtimes.len() < 2 {
        return 0.0;
    }

    let mean = runtimes.iter().sum::<f64>() / runtimes.len() as f64;
    let variance =
        runtimes.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (runtimes.len() - 1) as f64;

    variance.sqrt()
}

pub fn calculate_metrics_std_dev(
    round_metrics: &[ConsensusMetrics],
    avg_metrics: &ConsensusMetrics,
) -> MetricsStdDev {
    if round_metrics.len() < 2 {
        return MetricsStdDev {
            latency_std_dev: 0.0,
            throughput_std_dev: 0.0,
            commit_rate_std_dev: 0.0,
            error_rate_std_dev: 0.0,
        };
    }

    let latency_variance = round_metrics
        .iter()
        .map(|m| (m.avg_latency_ms - avg_metrics.avg_latency_ms).powi(2))
        .sum::<f64>()
        / (round_metrics.len() - 1) as f64;

    let throughput_variance = round_metrics
        .iter()
        .map(|m| (m.throughput_blocks_per_sec - avg_metrics.throughput_blocks_per_sec).powi(2))
        .sum::<f64>()
        / (round_metrics.len() - 1) as f64;

    let commit_rate_variance = round_metrics
        .iter()
        .map(|m| (m.commit_rate - avg_metrics.commit_rate).powi(2))
        .sum::<f64>()
        / (round_metrics.len() - 1) as f64;

    let error_rate_variance = round_metrics
        .iter()
        .map(|m| (m.error_rate - avg_metrics.error_rate).powi(2))
        .sum::<f64>()
        / (round_metrics.len() - 1) as f64;

    MetricsStdDev {
        latency_std_dev: latency_variance.sqrt(),
        throughput_std_dev: throughput_variance.sqrt(),
        commit_rate_std_dev: commit_rate_variance.sqrt(),
        error_rate_std_dev: error_rate_variance.sqrt(),
    }
}

pub fn calculate_average_metrics(round_metrics: &[ConsensusMetrics]) -> ConsensusMetrics {
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
            block_proposal_randomness: None,
            geographical_diversity: None,
            hashing_power_distribution: None,
            token_concentration: None,
            wealth_distribution: None,
            availability: 0.0,
            confirmation_latency_ms: 0.0,
            max_throughput_tps: 0.0,
            cost_of_attack: None,
            fault_tolerance: 0.0,
            reliability: 0.0,
            stale_block_rate: 0.0,
        };
    }

    let count = round_metrics.len() as f64;
    let strategy_name = round_metrics[0].strategy_name.clone();

    ConsensusMetrics {
        strategy_name,
        total_blocks: round_metrics[0].total_blocks,
        committed_blocks: (round_metrics
            .iter()
            .map(|m| m.committed_blocks)
            .sum::<usize>() as f64
            / count) as usize,
        failed_blocks: (round_metrics.iter().map(|m| m.failed_blocks).sum::<usize>() as f64 / count)
            as usize,
        error_blocks: (round_metrics.iter().map(|m| m.error_blocks).sum::<usize>() as f64 / count)
            as usize,
        min_latency_ms: round_metrics
            .iter()
            .map(|m| m.min_latency_ms)
            .min()
            .unwrap_or(0),
        max_latency_ms: round_metrics
            .iter()
            .map(|m| m.max_latency_ms)
            .max()
            .unwrap_or(0),
        avg_latency_ms: round_metrics.iter().map(|m| m.avg_latency_ms).sum::<f64>() / count,
        throughput_blocks_per_sec: round_metrics
            .iter()
            .map(|m| m.throughput_blocks_per_sec)
            .sum::<f64>()
            / count,
        error_rate: round_metrics.iter().map(|m| m.error_rate).sum::<f64>() / count,
        commit_rate: round_metrics.iter().map(|m| m.commit_rate).sum::<f64>() / count,
        data_integrity_maintained: round_metrics.iter().all(|m| m.data_integrity_maintained),
        // Extended metrics - average across rounds
        block_proposal_randomness: round_metrics[0].block_proposal_randomness,
        geographical_diversity: round_metrics[0].geographical_diversity,
        hashing_power_distribution: round_metrics[0].hashing_power_distribution,
        token_concentration: round_metrics[0].token_concentration,
        wealth_distribution: round_metrics[0].wealth_distribution,
        availability: round_metrics.iter().map(|m| m.availability).sum::<f64>() / count,
        confirmation_latency_ms: round_metrics.iter().map(|m| m.confirmation_latency_ms).sum::<f64>() / count,
        max_throughput_tps: round_metrics.iter().map(|m| m.max_throughput_tps).sum::<f64>() / count,
        cost_of_attack: round_metrics[0].cost_of_attack,
        fault_tolerance: round_metrics.iter().map(|m| m.fault_tolerance).sum::<f64>() / count,
        reliability: round_metrics.iter().map(|m| m.reliability).sum::<f64>() / count,
        stale_block_rate: round_metrics.iter().map(|m| m.stale_block_rate).sum::<f64>() / count,
    }
}
