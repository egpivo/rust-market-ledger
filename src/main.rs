mod pbft;
mod network;

use chrono::prelude::*;
use pbft::{MessageType, PBFTManager, PBFTMessage};
use network::{broadcast_message, NetworkHandler, start_server};
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use std::env;
use std::thread;
use actix_rt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarketData {
    asset: String,
    price: f32,
    source: String,
    timestamp: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub data: Vec<MarketData>,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
}

impl Block {
    pub fn calculate_hash(&self) -> String {
        let data_str = serde_json::to_string(&self.data).unwrap_or_default();
        let input = format!("{}{}{}{}{}", 
            self.index, self.timestamp, data_str, self.previous_hash, self.nonce);
        let mut hasher = Sha256::new();
        hasher.update(input);
        format!("{:x}", hasher.finalize())
    }
    
    pub fn calculate_hash_with_nonce(&mut self) {
        self.hash = self.calculate_hash();
    }
}

#[derive(Deserialize, Debug)]
struct CoinGeckoResponse {
    bitcoin: PriceDetail,
}

#[derive(Deserialize, Debug)]
struct PriceDetail {
    usd: f32,
}

struct DatabaseManager {
    conn_str: String,
}

impl DatabaseManager {
    fn new(path: &str) -> Self {
        DatabaseManager { conn_str: path.to_string() }
    }

    fn init(&self) -> SqlResult<()> {
        let conn = Connection::open(&self.conn_str)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blockchain (
                id            INTEGER PRIMARY KEY,
                block_index   INTEGER NOT NULL,
                timestamp     INTEGER NOT NULL,
                data_json     TEXT NOT NULL,
                prev_hash     TEXT NOT NULL,
                hash          TEXT NOT NULL,
                nonce         INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    fn save_block(&self, block: &Block) -> SqlResult<()> {
        let conn = Connection::open(&self.conn_str)?;
        let data_json = serde_json::to_string(&block.data).unwrap();
        
        conn.execute(
            "INSERT INTO blockchain (block_index, timestamp, data_json, prev_hash, hash, nonce)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                block.index, 
                block.timestamp, 
                data_json, 
                block.previous_hash, 
                block.hash, 
                block.nonce
            ],
        )?;
        println!("[Database] Block #{} saved to SQLite.", block.index);
        Ok(())
    }

    fn query_latest_blocks(&self, limit: u64) -> SqlResult<()> {
        let conn = Connection::open(&self.conn_str)?;
        let mut stmt = conn.prepare("SELECT block_index, hash, data_json FROM blockchain ORDER BY block_index DESC LIMIT ?")?;
        
        let rows = stmt.query_map([limit], |row| {
            let idx: u64 = row.get(0)?;
            let hash: String = row.get(1)?;
            let data: String = row.get(2)?;
            Ok((idx, hash, data))
        })?;

        println!("\n[Audit] Verifying latest blocks in DB:");
        for row in rows {
            let (idx, hash, data) = row?;
            println!("   Block #{} | Hash: {}... | Data: {:.50}...", idx, &hash[0..8.min(hash.len())], data);
        }
        Ok(())
    }

    pub fn get_block_count(&self) -> SqlResult<u64> {
        let conn = Connection::open(&self.conn_str)?;
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM blockchain", [], |row| row.get(0))?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_block_hash_calculation() {
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
        let test_db = "test_blockchain.db";
        let db = DatabaseManager::new(test_db);
        assert!(db.init().is_ok());
        
        let count = db.get_block_count().unwrap();
        assert_eq!(count, 0);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_database_save_and_retrieve_block() {
        let test_db = "test_blockchain_save.db";
        let db = DatabaseManager::new(test_db);
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
        
        fs::remove_file(test_db).ok();
    }
}

async fn fetch_bitcoin_price_offline() -> Result<MarketData, Box<dyn Error>> {
    let timestamp = Utc::now().timestamp();
    let base_price = 50000.0;
    let variation = (timestamp % 1000) as f32 / 10.0;
    Ok(MarketData {
        asset: "BTC".to_string(),
        price: base_price + variation,
        source: "MockData".to_string(),
        timestamp,
    })
}

async fn fetch_bitcoin_price() -> Result<MarketData, Box<dyn Error>> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
    let max_retries = 3;
    let mut last_error = None;
    
    let client = reqwest::Client::builder()
        .user_agent("rust-market-ledger/0.1.0")
        .timeout(Duration::from_secs(10))
        .build()?;
    
    for attempt in 1..=max_retries {
        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    last_error = Some(format!("HTTP status: {}", status));
                    if status == 429 || status == 403 {
                        let delay_ms = 1000 * attempt as u64;
                        if attempt < max_retries {
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                    } else if attempt < max_retries {
                        tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                        continue;
                    }
                    return Err(format!("API returned status: {}", status).into());
                }
                
                match response.json::<CoinGeckoResponse>().await {
                    Ok(resp) => {
                        return Ok(MarketData {
                            asset: "BTC".to_string(),
                            price: resp.bitcoin.usd,
                            source: "CoinGecko".to_string(),
                            timestamp: Utc::now().timestamp(),
                        });
                    }
                    Err(e) => {
                        last_error = Some(format!("JSON decode error: {}", e));
                        if attempt < max_retries {
                            tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                            continue;
                        }
                    }
                }
            }
            Err(e) => {
                last_error = Some(format!("Request error: {}", e));
                if attempt < max_retries {
                    tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                    continue;
                }
            }
        }
    }
    
    Err(format!("Failed after {} attempts. Last error: {}", max_retries, last_error.unwrap_or_default()).into())
}

async fn run_pbft_consensus(
    block: Block,
    pbft: Arc<PBFTManager>,
    node_addresses: &[String],
    port: u16,
) -> Result<Option<Block>, Box<dyn Error>> {
    let sequence = block.index;
    
    if pbft.is_primary(sequence) {
        println!("[PBFT] Node {} is PRIMARY for block #{}", pbft.node_id(), sequence);
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
        println!("[PBFT] Waiting for Prepare quorum...");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    
    let commit_msg = pbft.create_commit(&block.hash, sequence);
    broadcast_message(&commit_msg, node_addresses, port).await;
    let commit_quorum = pbft.handle_commit(&commit_msg);
    
    if commit_quorum {
        println!("[PBFT] Block #{} reached COMMIT quorum!", sequence);
        tokio::time::sleep(Duration::from_millis(300)).await;
        return Ok(Some(block));
    }
    
    println!("[PBFT] Block #{} failed to reach commit quorum", sequence);
    Ok(None)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
    
    println!("[Node {}] Starting on port {}", node_id, port);
    println!("[Network] Total nodes: {}", total_nodes);
    
    let db_path = format!("blockchain_node_{}.db", node_id);
    let db = DatabaseManager::new(&db_path);
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
    
    let mut last_hash = String::from("0000_genesis_hash");
    let mut last_index = 0;
    
    for round in 0..3 {
        println!("\n{}", "=".repeat(60));
        println!("Round {}: Starting ETL + PBFT Consensus", round + 1);
        
        let price_result = if use_offline {
            fetch_bitcoin_price_offline().await
        } else {
            fetch_bitcoin_price().await
        };
        
        match price_result {
            Ok(market_data) => {
                println!("[Extract] Price: ${}", market_data.price);
                
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
                
                println!("[Transform] Block #{} created", new_block.index);
                
                match run_pbft_consensus(new_block.clone(), pbft.clone(), &node_addresses, port).await {
                    Ok(Some(committed_block)) => {
                        if let Err(e) = db.save_block(&committed_block) {
                            eprintln!("[Error] Database Error: {}", e);
                        } else {
                            last_hash = committed_block.hash.clone();
                            println!("[Load] Block #{} committed and saved!", committed_block.index);
                        }
                    }
                    Ok(None) => {
                        eprintln!("[Warning] [PBFT] Consensus failed for block #{}", new_block.index);
                        last_index -= 1;
                    }
                    Err(e) => {
                        eprintln!("[Error] [PBFT] Error: {}", e);
                        last_index -= 1;
                    }
                }
            }
            Err(e) => eprintln!("[Warning] [Extract] Fetch Error: {}", e),
        }
        
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    
    println!("\n{}", "=".repeat(60));
    db.query_latest_blocks(5)?;
    
    println!("\n[Success] Node {} completed successfully!", node_id);
    
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    Ok(())
}
