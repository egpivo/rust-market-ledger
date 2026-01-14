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

// ==========================================
// 1. Data Models (資料模型)
// ==========================================

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

// 接收 API 回傳格式
#[derive(Deserialize, Debug)]
struct CoinGeckoResponse {
    bitcoin: PriceDetail,
}
#[derive(Deserialize, Debug)]
struct PriceDetail {
    usd: f32,
}

// ==========================================
// 2. Database Layer (SQLite 持久化)
// ==========================================

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
        println!("💾 [Database] Block #{} saved to SQLite.", block.index);
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

        println!("\n🔍 [Audit] Verifying latest blocks in DB:");
        for row in rows {
            let (idx, hash, data) = row?;
            println!("   Block #{} | Hash: {}... | Data: {:.50}...", idx, &hash[0..8.min(hash.len())], data);
        }
        Ok(())
    }
}

// ==========================================
// 3. Consensus & Logic
// ==========================================

async fn fetch_bitcoin_price() -> Result<MarketData, Box<dyn Error>> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
    let resp = reqwest::get(url).await?.json::<CoinGeckoResponse>().await?;
    Ok(MarketData {
        asset: "BTC".to_string(),
        price: resp.bitcoin.usd,
        source: "CoinGecko".to_string(),
        timestamp: Utc::now().timestamp(),
    })
}

// ==========================================
// 4. PBFT Consensus Integration
// ==========================================

async fn run_pbft_consensus(
    block: Block,
    pbft: Arc<PBFTManager>,
    node_addresses: &[String],
    port: u16,
) -> Result<Option<Block>, Box<dyn Error>> {
    let sequence = block.index;
    
    // Phase 1: PrePrepare (主節點發起)
    if pbft.is_primary(sequence) {
        println!("🎯 [PBFT] Node {} is PRIMARY for block #{}", pbft.node_id(), sequence);
        let block_json = serde_json::to_string(&block).unwrap_or_default();
        let pre_prepare_msg = pbft.create_pre_prepare(&block.hash, &block_json, sequence);
        
        // 廣播 PrePrepare
        broadcast_message(&pre_prepare_msg, node_addresses, port).await;
        
        // 主節點自己處理 PrePrepare
        pbft.handle_pre_prepare(&pre_prepare_msg);
    }
    
    // Phase 2: Prepare (所有節點)
    tokio::time::sleep(Duration::from_millis(500)).await; // 等待 PrePrepare 傳播
    
    let prepare_msg = pbft.create_prepare(&block.hash, sequence);
    broadcast_message(&prepare_msg, node_addresses, port).await;
    let prepare_quorum = pbft.handle_prepare(&prepare_msg);
    
    if !prepare_quorum {
        println!("⏳ [PBFT] Waiting for Prepare quorum...");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    
    // Phase 3: Commit (所有節點)
    let commit_msg = pbft.create_commit(&block.hash, sequence);
    broadcast_message(&commit_msg, node_addresses, port).await;
    let commit_quorum = pbft.handle_commit(&commit_msg);
    
    if commit_quorum {
        println!("✅ [PBFT] Block #{} reached COMMIT quorum!", sequence);
        tokio::time::sleep(Duration::from_millis(300)).await;
        return Ok(Some(block));
    }
    
    println!("❌ [PBFT] Block #{} failed to reach commit quorum", sequence);
    Ok(None)
}

// ==========================================
// 5. Main Flow
// ==========================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 解析命令行參數
    let args: Vec<String> = env::args().collect();
    let node_id: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8000 + node_id as u16);
    
    // 節點配置 (4 個節點)
    let node_addresses = vec![
        "127.0.0.1:8000".to_string(),
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];
    let total_nodes = node_addresses.len();
    
    println!("🚀 [Node {}] Starting on port {}", node_id, port);
    println!("📡 [Network] Total nodes: {}", total_nodes);
    
    // 1. Setup Database
    let db_path = format!("blockchain_node_{}.db", node_id);
    let db = DatabaseManager::new(&db_path);
    db.init()?;
    
    // 2. Setup PBFT
    let pbft = Arc::new(PBFTManager::new(node_id, total_nodes, node_addresses.clone()));
    let pbft_clone = pbft.clone();
    
    // 3. Setup Network Handler
    let network_handler = Arc::new(NetworkHandler::new(move |msg: PBFTMessage| {
        let pbft = pbft_clone.clone();
        match msg.msg_type {
            MessageType::PrePrepare => pbft.handle_pre_prepare(&msg),
            MessageType::Prepare => pbft.handle_prepare(&msg),
            MessageType::Commit => pbft.handle_commit(&msg),
        }
    }));
    
    // 4. Start HTTP Server (背景執行)
    let server_port = port;
    let handler_for_server = network_handler.clone();
    tokio::spawn(async move {
        if let Err(e) = start_server(server_port, handler_for_server).await {
            eprintln!("❌ Server error: {}", e);
        }
    });
    
    // 等待伺服器啟動
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 5. 主循環：ETL + PBFT 共識
    let mut last_hash = String::from("0000_genesis_hash");
    let mut last_index = 0;
    
    // 只運行 3 個區塊用於演示
    for round in 0..3 {
        println!("\n{}", "=".repeat(60));
        println!("📦 Round {}: Starting ETL + PBFT Consensus", round + 1);
        
        // --- Step 1: Extract (Fetch) ---
        match fetch_bitcoin_price().await {
            Ok(market_data) => {
                println!("📊 [Extract] Price: ${}", market_data.price);
                
                // --- Step 2: Transform (Create Block) ---
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
                
                println!("🔨 [Transform] Block #{} created", new_block.index);
                
                // --- Step 3: PBFT Consensus ---
                match run_pbft_consensus(new_block.clone(), pbft.clone(), &node_addresses, port).await {
                    Ok(Some(committed_block)) => {
                        // --- Step 4: Load (Persist to DB) ---
                        if let Err(e) = db.save_block(&committed_block) {
                            eprintln!("❌ Database Error: {}", e);
                        } else {
                            last_hash = committed_block.hash.clone();
                            println!("✨ [Load] Block #{} committed and saved!", committed_block.index);
                        }
                    }
                    Ok(None) => {
                        eprintln!("⚠️  [PBFT] Consensus failed for block #{}", new_block.index);
                        last_index -= 1; // 回退索引
                    }
                    Err(e) => {
                        eprintln!("❌ [PBFT] Error: {}", e);
                        last_index -= 1;
                    }
                }
            }
            Err(e) => eprintln!("⚠️  [Extract] Fetch Error: {}", e),
        }
        
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    
    // 最終驗證
    println!("\n{}", "=".repeat(60));
    db.query_latest_blocks(5)?;
    
    println!("\n✅ Node {} completed successfully!", node_id);
    
    // 保持伺服器運行
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    Ok(())
}
