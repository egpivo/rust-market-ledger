mod consensus;
mod etl;
mod logger;
mod network;

use actix_rt;
use chrono::prelude::*;
use consensus::algorithms::{eventual, flexible_paxos, gossip, pbft::PBFTConsensus, quorumless};
use consensus::algorithms::{MessageType, PBFTManager, PBFTMessage};
use consensus::{ConsensusAlgorithm, ConsensusResult};
use etl::extract::Extractor;
use etl::load::DatabaseManager;
use etl::transform::Transformer;
use etl::{Block, MarketData};
use network::{broadcast_message, start_server, NetworkHandler};
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    static INIT: std::sync::Once = std::sync::Once::new();

    fn init() {
        INIT.call_once(|| {
            logger::init_test_logger();
        });
    }

    #[test]
    fn test_block_hash_calculation() {
        init();
        let block = Block {
            index: 1,
            timestamp: 1234567890,
            data: vec![MarketData {
                asset: "BTC".to_string(),
                price: 50000.0,
                source: "Test".to_string(),
                timestamp: 1234567890,
            }],
            previous_hash: "0000_genesis".to_string(),
            hash: String::new(),
            nonce: 0,
        };

        let hash = block.calculate_hash();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_block_hash_consistency() {
        init();
        let block1 = Block {
            index: 1,
            timestamp: 1234567890,
            data: vec![MarketData {
                asset: "BTC".to_string(),
                price: 50000.0,
                source: "Test".to_string(),
                timestamp: 1234567890,
            }],
            previous_hash: "0000_genesis".to_string(),
            hash: String::new(),
            nonce: 0,
        };

        let block2 = block1.clone();
        assert_eq!(block1.calculate_hash(), block2.calculate_hash());
    }

    #[test]
    fn test_database_manager_init() {
        init();
        let test_db = "test_blockchain.db";
        let db = DatabaseManager::new(test_db).unwrap();
        assert!(db.init().is_ok());

        let count = db.get_block_count().unwrap();
        assert_eq!(count, 0);

        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_database_save_and_retrieve_block() {
        init();
        let test_db = "test_blockchain_save.db";
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();

        let block = Block {
            index: 1,
            timestamp: 1234567890,
            data: vec![MarketData {
                asset: "BTC".to_string(),
                price: 50000.0,
                source: "Test".to_string(),
                timestamp: 1234567890,
            }],
            previous_hash: "0000_genesis".to_string(),
            hash: "abc123".to_string(),
            nonce: 0,
        };

        assert!(db.save_block(&block).is_ok());

        let count = db.get_block_count().unwrap();
        assert_eq!(count, 1);

        let retrieved = db.get_block_by_index(1).unwrap();
        assert_eq!(retrieved.index, 1);
        assert_eq!(retrieved.hash, "abc123");

        // Test get by hash
        let retrieved_by_hash = db.get_block_by_hash("abc123").unwrap();
        assert_eq!(retrieved_by_hash.index, 1);

        let latest = db.get_latest_block().unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().index, 1);

        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_database_batch_operations() {
        init();
        let test_db = "test_blockchain_batch.db";
        // Clean up any existing test database
        fs::remove_file(test_db).ok();

        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();

        let mut block1 = Block {
            index: 1,
            timestamp: 1234567890,
            data: vec![MarketData {
                asset: "BTC".to_string(),
                price: 50000.0,
                source: "Test".to_string(),
                timestamp: 1234567890,
            }],
            previous_hash: "0000_genesis".to_string(),
            hash: String::new(),
            nonce: 0,
        };
        block1.calculate_hash_with_nonce();

        let mut block2 = Block {
            index: 2,
            timestamp: 1234567891,
            data: vec![MarketData {
                asset: "BTC".to_string(),
                price: 50100.0,
                source: "Test".to_string(),
                timestamp: 1234567891,
            }],
            previous_hash: block1.hash.clone(),
            hash: String::new(),
            nonce: 0,
        };
        block2.calculate_hash_with_nonce();

        let blocks = vec![block1, block2];

        let saved = db.save_blocks(&blocks).unwrap();
        assert_eq!(saved, 2);

        let count = db.get_block_count().unwrap();
        assert_eq!(count, 2);

        // Test range query
        let range_blocks = db.get_blocks_range(1, 2).unwrap();
        assert_eq!(range_blocks.len(), 2);

        let is_valid = db.verify_chain().unwrap();
        assert!(is_valid);

        fs::remove_file(test_db).ok();
    }
}

/// Consensus algorithm selection
#[derive(Debug, Clone, Copy, PartialEq)]
enum ConsensusType {
    PBFT,
    Gossip,
    Eventual,
    Quorumless,
    FlexiblePaxos,
}

impl ConsensusType {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pbft" | "1" => Some(ConsensusType::PBFT),
            "gossip" | "2" => Some(ConsensusType::Gossip),
            "eventual" | "3" => Some(ConsensusType::Eventual),
            "quorumless" | "4" => Some(ConsensusType::Quorumless),
            "flexible_paxos" | "flexiblepaxos" | "fpaxos" | "paxos" | "5" => {
                Some(ConsensusType::FlexiblePaxos)
            }
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            ConsensusType::PBFT => "PBFT",
            ConsensusType::Gossip => "Gossip",
            ConsensusType::Eventual => "Eventual Consistency",
            ConsensusType::Quorumless => "Quorum-less (Weighted)",
            ConsensusType::FlexiblePaxos => "Flexible Paxos",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ConsensusType::PBFT => {
                "Byzantine fault tolerance with majority voting (2f+1 out of 3f+1)"
            }
            ConsensusType::Gossip => "Epidemic/gossip protocol, no majority voting required",
            ConsensusType::Eventual => "Time-based commitment, no majority voting required",
            ConsensusType::Quorumless => {
                "Weighted voting based on node reputation, no majority voting"
            }
            ConsensusType::FlexiblePaxos => {
                "Flexible quorum Paxos: Q1 (phase-1) intersects with previous Q2 (phase-2)"
            }
        }
    }
}

fn show_consensus_menu() {
    println!("\n{}", "=".repeat(70));
    println!("  Consensus Algorithm Selection");
    println!("{}", "=".repeat(70));
    println!();
    println!("  1. PBFT (Practical Byzantine Fault Tolerance)");
    println!("     - {}", ConsensusType::PBFT.description());
    println!();
    println!("  2. Gossip Protocol");
    println!("     - {}", ConsensusType::Gossip.description());
    println!();
    println!("  3. Eventual Consistency");
    println!("     - {}", ConsensusType::Eventual.description());
    println!();
    println!("  4. Quorum-less (Weighted Voting)");
    println!("     - {}", ConsensusType::Quorumless.description());
    println!();
    println!("  5. Flexible Paxos");
    println!("     - {}", ConsensusType::FlexiblePaxos.description());
    println!();
    println!("{}", "=".repeat(70));
    print!("\n  Select consensus algorithm (1-5) or press Enter for PBFT (default): ");
    io::stdout().flush().unwrap();
}

fn get_consensus_selection() -> ConsensusType {
    let args: Vec<String> = env::args().collect();
    for arg in &args {
        if arg.starts_with("--consensus=") {
            if let Some(value) = arg.split('=').nth(1) {
                if let Some(consensus) = ConsensusType::from_str(value) {
                    return consensus;
                }
            }
        }
        if arg == "--consensus" || arg == "-c" {
            if let Some(next_arg) = args.iter().skip_while(|a| a != &arg).nth(1) {
                if let Some(consensus) = ConsensusType::from_str(next_arg) {
                    return consensus;
                }
            }
        }
    }

    show_consensus_menu();

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            let trimmed = input.trim();
            if trimmed.is_empty() {
                ConsensusType::PBFT // Default
            } else {
                ConsensusType::from_str(trimmed).unwrap_or(ConsensusType::PBFT)
            }
        }
        Err(_) => ConsensusType::PBFT, // Default on error
    }
}

async fn run_pbft_consensus(
    block: Block,
    pbft: Arc<PBFTManager>,
    node_addresses: &[String],
    port: u16,
) -> Result<Option<Block>, Box<dyn Error>> {
    let sequence = block.index;

    if pbft.is_primary(sequence) {
        info!(
            node_id = pbft.node_id(),
            block_index = sequence,
            "PBFT: Node is PRIMARY for block"
        );
        let block_json = serde_json::to_string(&block).unwrap_or_default();
        let pre_prepare_msg = pbft.create_pre_prepare(&block.hash, &block_json, sequence);

        broadcast_message(&pre_prepare_msg, node_addresses, port).await;
        pbft.handle_pre_prepare(&pre_prepare_msg);
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    let prepare_msg = pbft.create_prepare(&block.hash, sequence);
    broadcast_message(&prepare_msg, node_addresses, port).await;
    let prepare_quorum = pbft.handle_prepare(&prepare_msg);

    if !prepare_quorum {
        debug!(block_index = sequence, "PBFT: Waiting for Prepare quorum");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let commit_msg = pbft.create_commit(&block.hash, sequence);
    broadcast_message(&commit_msg, node_addresses, port).await;
    let commit_quorum = pbft.handle_commit(&commit_msg);

    if commit_quorum {
        info!(block_index = sequence, "PBFT: Block reached COMMIT quorum");
        tokio::time::sleep(Duration::from_millis(300)).await;
        return Ok(Some(block));
    }

    warn!(
        block_index = sequence,
        "PBFT: Block failed to reach commit quorum"
    );
    Ok(None)
}

async fn run_consensus(
    consensus_type: ConsensusType,
    block: Block,
    node_id: usize,
    total_nodes: usize,
    node_addresses: &[String],
    port: u16,
    pbft: Arc<PBFTManager>,
) -> Result<Option<Block>, Box<dyn Error>> {
    match consensus_type {
        ConsensusType::PBFT => run_pbft_consensus(block, pbft, node_addresses, port).await,
        ConsensusType::Gossip => {
            let consensus = Arc::new(gossip::GossipConsensus::new(node_id, 3, 2));
            match consensus.propose(&block).await {
                Ok(ConsensusResult::Committed(_)) => {
                    info!(block_index = block.index, "Gossip: Block committed");
                    Ok(Some(block))
                }
                Ok(ConsensusResult::Pending) => {
                    warn!(block_index = block.index, "Gossip: Block pending");
                    Ok(None)
                }
                Ok(ConsensusResult::Rejected(reason)) => {
                    warn!(block_index = block.index, reason = %reason, "Gossip: Block rejected");
                    Ok(None)
                }
                Err(e) => Err(e),
            }
        }
        ConsensusType::Eventual => {
            let consensus = Arc::new(eventual::EventualConsensus::new(node_id, 1000, 2));
            match consensus.propose(&block).await {
                Ok(ConsensusResult::Committed(_)) => {
                    info!(block_index = block.index, "Eventual: Block committed");
                    Ok(Some(block))
                }
                Ok(ConsensusResult::Pending) => {
                    warn!(block_index = block.index, "Eventual: Block pending");
                    Ok(None)
                }
                Ok(ConsensusResult::Rejected(reason)) => {
                    warn!(block_index = block.index, reason = %reason, "Eventual: Block rejected");
                    Ok(None)
                }
                Err(e) => Err(e),
            }
        }
        ConsensusType::Quorumless => {
            let consensus = Arc::new(quorumless::QuorumlessConsensus::new(node_id, 5.0));
            consensus.set_node_weight(0, 2.0);
            consensus.set_node_weight(1, 2.0);
            consensus.set_node_weight(2, 1.5);
            consensus.set_node_weight(3, 1.5);

            match consensus.propose(&block).await {
                Ok(ConsensusResult::Committed(_)) => {
                    info!(block_index = block.index, "Quorumless: Block committed");
                    Ok(Some(block))
                }
                Ok(ConsensusResult::Pending) => {
                    warn!(
                        block_index = block.index,
                        "Quorumless: Block pending (need more votes)"
                    );
                    Ok(None)
                }
                Ok(ConsensusResult::Rejected(reason)) => {
                    warn!(block_index = block.index, reason = %reason, "Quorumless: Block rejected");
                    Ok(None)
                }
                Err(e) => Err(e),
            }
        }
        ConsensusType::FlexiblePaxos => {
            let q1_size = (total_nodes + 1) / 2 + 1;
            let q2_size = total_nodes / 2;
            let consensus = Arc::new(flexible_paxos::FlexiblePaxos::new(
                node_id,
                total_nodes,
                q1_size,
                q2_size,
            ));

            match consensus.propose(&block).await {
                Ok(ConsensusResult::Committed(committed_block)) => {
                    info!(
                        block_index = committed_block.index,
                        q1 = q1_size,
                        q2 = q2_size,
                        "Flexible Paxos: Block committed"
                    );
                    Ok(Some(committed_block))
                }
                Ok(ConsensusResult::Pending) => {
                    warn!(
                        block_index = block.index,
                        "Flexible Paxos: Block pending (quorum not reached)"
                    );
                    Ok(None)
                }
                Ok(ConsensusResult::Rejected(reason)) => {
                    warn!(block_index = block.index, reason = %reason, "Flexible Paxos: Block rejected");
                    Ok(None)
                }
                Err(e) => Err(e),
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logger::init_logger_detailed();

    let consensus_type = get_consensus_selection();
    info!(
        consensus = consensus_type.name(),
        description = consensus_type.description(),
        "Selected consensus algorithm"
    );

    let args: Vec<String> = env::args().collect();
    let node_id: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let port: u16 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8000 + node_id as u16);
    let use_offline = args.contains(&"--offline".to_string()) || args.contains(&"-o".to_string());

    let node_addresses = vec![
        "127.0.0.1:8000".to_string(),
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];
    let total_nodes = node_addresses.len();

    let memory = logger::get_memory_usage_public();
    info!(
        hostname = %logger::get_hostname(),
        memory = %memory,
        "Node {} starting on port {}", node_id, port
    );
    info!("Network: {} total nodes", total_nodes);

    let db_path = format!("blockchain_node_{}.db", node_id);
    let db = DatabaseManager::new(&db_path)?;
    db.init()?;

    // Initialize PBFT (always needed for network server, even if not used for consensus)
    let pbft = Arc::new(PBFTManager::new(
        node_id,
        total_nodes,
        node_addresses.clone(),
    ));
    let pbft_clone = pbft.clone();

    let network_handler = Arc::new(NetworkHandler::new(move |msg: PBFTMessage| {
        let pbft = pbft_clone.clone();
        match msg.msg_type {
            MessageType::PrePrepare => pbft.handle_pre_prepare(&msg),
            MessageType::Prepare => pbft.handle_prepare(&msg),
            MessageType::Commit => pbft.handle_commit(&msg),
        }
    }));

    let server_port = port;
    let handler_for_server = network_handler.clone();

    if consensus_type == ConsensusType::PBFT {
        thread::spawn(move || {
            actix_rt::System::new().block_on(async {
                let _ = start_server(server_port, handler_for_server).await;
            });
        });
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Initialize ETL components
    let extractor = Extractor::new()?;
    let transformer = Transformer::new();

    let mut last_hash = String::from("0000_genesis_hash");
    let mut last_index = 0u64;
    let mut last_timestamp: Option<i64> = None;

    if let Ok(Some(latest_block)) = db.get_latest_block() {
        last_hash = latest_block.hash.clone();
        last_index = latest_block.index;
        last_timestamp = Some(latest_block.timestamp);
        info!(
            block_index = last_index,
            hash_preview = &last_hash[0..8.min(last_hash.len())],
            "ETL: Loaded previous block"
        );
    }

    for round in 0..3 {
        info!("{}", "=".repeat(60));
        info!(
            round = round + 1,
            consensus = consensus_type.name(),
            "Starting ETL + Consensus"
        );

        let extract_result = if use_offline {
            extractor.extract_offline().await
        } else {
            extractor.extract_from_api().await
        };

        match extract_result {
            Ok(extract_data) => {
                info!(
                    price = extract_data.price,
                    source = %extract_data.source,
                    timestamp = extract_data.timestamp,
                    "Extract: Market data retrieved"
                );

                let transform_result = transformer.transform(
                    extract_data.price,
                    extract_data.timestamp,
                    extract_data.source.clone(),
                    last_timestamp,
                );

                match transform_result {
                    Ok(transformed_data) => {
                        if transformed_data.is_deduplicated {
                            warn!(
                                window_seconds = transformer.deduplication_window_seconds(),
                                "Transform: Data appears to be duplicate, skipping"
                            );
                            continue;
                        }

                        let normalized_price = transformer.normalize_price(transformed_data.price);

                        debug!(
                            asset = %transformed_data.asset,
                            price = transformed_data.price,
                            normalized_price = normalized_price,
                            "Transform: Data transformed and normalized"
                        );

                        let market_data = MarketData {
                            asset: transformed_data.asset,
                            price: normalized_price,
                            source: transformed_data.source,
                            timestamp: transformed_data.timestamp,
                        };

                        last_index += 1;
                        let mut new_block = Block {
                            index: last_index,
                            timestamp: Utc::now().timestamp(),
                            data: vec![market_data],
                            previous_hash: last_hash.clone(),
                            hash: String::new(),
                            nonce: 0,
                        };
                        new_block.calculate_hash_with_nonce();

                        info!(
                            block_index = new_block.index,
                            hash_preview = &new_block.hash[0..8.min(new_block.hash.len())],
                            "Transform: Block created"
                        );

                        match run_consensus(
                            consensus_type,
                            new_block.clone(),
                            node_id,
                            total_nodes,
                            &node_addresses,
                            port,
                            pbft.clone(),
                        )
                        .await
                        {
                            Ok(Some(committed_block)) => match db.save_block(&committed_block) {
                                Ok(_) => {
                                    last_hash = committed_block.hash.clone();
                                    last_timestamp = Some(committed_block.timestamp);
                                    info!(
                                        block_index = committed_block.index,
                                        consensus = consensus_type.name(),
                                        "Load: Block committed and saved"
                                    );
                                }
                                Err(e) => {
                                    error!(error = %e, "Load: Database error");
                                    last_index -= 1;
                                }
                            },
                            Ok(None) => {
                                warn!(
                                    block_index = new_block.index,
                                    consensus = consensus_type.name(),
                                    "Consensus failed or pending"
                                );
                                last_index -= 1;
                            }
                            Err(e) => {
                                error!(
                                    error = %e,
                                    consensus = consensus_type.name(),
                                    "Error during consensus"
                                );
                                last_index -= 1;
                            }
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "Transform: Validation/Transformation error");
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Extract: Fetch error");
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    info!("{}", "=".repeat(60));
    db.print_latest_blocks(5)?;

    info!(node_id = node_id, "Node completed successfully");

    tokio::time::sleep(Duration::from_secs(5)).await;

    Ok(())
}
