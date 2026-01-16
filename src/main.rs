mod network;
mod etl;
mod consensus;
mod logger;

use chrono::prelude::*;
use consensus::algorithms::{MessageType, PBFTManager, PBFTMessage};
use network::{broadcast_message, NetworkHandler, start_server};
use etl::{Block, MarketData};
use etl::load::DatabaseManager;
use etl::extract::Extractor;
use etl::transform::Transformer;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use std::env;
use std::thread;
use actix_rt;
use tracing::{info, warn, error, debug};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    // Initialize logger for tests (only once)
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
        
        // Test retrieval
        let retrieved = db.get_block_by_index(1).unwrap();
        assert_eq!(retrieved.index, 1);
        assert_eq!(retrieved.hash, "abc123");
        
        // Test get by hash
        let retrieved_by_hash = db.get_block_by_hash("abc123").unwrap();
        assert_eq!(retrieved_by_hash.index, 1);
        
        // Test latest block
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
        
        // Test chain verification
        let is_valid = db.verify_chain().unwrap();
        assert!(is_valid);
        
        fs::remove_file(test_db).ok();
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
        info!(node_id = pbft.node_id(), block_index = sequence, "PBFT: Node is PRIMARY for block");
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
    
    warn!(block_index = sequence, "PBFT: Block failed to reach commit quorum");
    Ok(None)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger first (before any other operations)
    // Use detailed format for demo (includes hostname and memory)
    logger::init_logger_detailed();
    
    let args: Vec<String> = env::args().collect();
    let node_id: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8000 + node_id as u16);
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
    
    let pbft = Arc::new(PBFTManager::new(node_id, total_nodes, node_addresses.clone()));
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
    
    thread::spawn(move || {
        actix_rt::System::new().block_on(async {
            let _ = start_server(server_port, handler_for_server).await;
        });
    });
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Initialize ETL components
    let extractor = Extractor::new()?;
    let transformer = Transformer::new();
    
    // Get last block info for chain linking and deduplication
    let mut last_hash = String::from("0000_genesis_hash");
    let mut last_index = 0u64;
    let mut last_timestamp: Option<i64> = None;
    
    // Try to load last block from database
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
        info!(round = round + 1, "Starting ETL + PBFT Consensus");
        
        // Extract: Get market data from API or offline source
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
                
                // Transform: Validate and process the data
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
                        
                        // Normalize price if needed
                        let normalized_price = transformer.normalize_price(transformed_data.price);
                        
                        debug!(
                            asset = %transformed_data.asset,
                            price = transformed_data.price,
                            normalized_price = normalized_price,
                            "Transform: Data transformed and normalized"
                        );
                        
                        // Create MarketData from transformed result
                        let market_data = MarketData {
                            asset: transformed_data.asset,
                            price: normalized_price,
                            source: transformed_data.source,
                            timestamp: transformed_data.timestamp,
                        };
                        
                        // Create block
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
                        
                        // PBFT Consensus
                        match run_pbft_consensus(new_block.clone(), pbft.clone(), &node_addresses, port).await {
                            Ok(Some(committed_block)) => {
                                // Load: Save to database
                                match db.save_block(&committed_block) {
                                    Ok(_) => {
                                        last_hash = committed_block.hash.clone();
                                        last_timestamp = Some(committed_block.timestamp);
                                        info!(
                                            block_index = committed_block.index,
                                            "Load: Block committed and saved"
                                        );
                                    }
                                    Err(e) => {
                                        error!(error = %e, "Load: Database error");
                                        last_index -= 1;
                                    }
                                }
                            }
                            Ok(None) => {
                                warn!(block_index = new_block.index, "PBFT: Consensus failed");
                                last_index -= 1;
                            }
                            Err(e) => {
                                error!(error = %e, "PBFT: Error during consensus");
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
