use crate::etl::Block;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use tracing::{info, debug};

/// Custom error type for database operations
#[derive(Debug)]
pub enum DatabaseError {
    Sqlite(rusqlite::Error),
    Serialization(String),
    NotFound(String),
    InvalidData(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::Sqlite(e) => write!(f, "SQLite error: {}", e),
            DatabaseError::Serialization(e) => write!(f, "Serialization error: {}", e),
            DatabaseError::NotFound(e) => write!(f, "Not found: {}", e),
            DatabaseError::InvalidData(e) => write!(f, "Invalid data: {}", e),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        DatabaseError::Sqlite(err)
    }
}

/// Result type for database operations
pub type DbResult<T> = Result<T, DatabaseError>;

/// Database manager with connection pooling and enhanced features
pub struct DatabaseManager {
    conn: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    /// Create a new DatabaseManager instance
    pub fn new(path: &str) -> DbResult<Self> {
        let conn = Connection::open(path)?;
        Ok(DatabaseManager {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Initialize the database schema with indexes for better performance
    pub fn init(&self) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        
        // Create main blockchain table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blockchain (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                block_index   INTEGER NOT NULL UNIQUE,
                timestamp     INTEGER NOT NULL,
                data_json     TEXT NOT NULL,
                prev_hash     TEXT NOT NULL,
                hash          TEXT NOT NULL UNIQUE,
                nonce         INTEGER NOT NULL,
                created_at    INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;

        // Create indexes for better query performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_block_index ON blockchain(block_index)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_hash ON blockchain(hash)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON blockchain(timestamp)",
            [],
        )?;

        Ok(())
    }

    /// Save a single block to the database
    pub fn save_block(&self, block: &Block) -> DbResult<()> {
        let conn = self.conn.lock().unwrap();
        let data_json = serde_json::to_string(&block.data)
            .map_err(|e| DatabaseError::Serialization(e.to_string()))?;

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
        
        info!(block_index = block.index, "Database: Block saved to SQLite");
        Ok(())
    }

    /// Save multiple blocks in a transaction (batch operation)
    pub fn save_blocks(&self, blocks: &[Block]) -> DbResult<usize> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        
        let mut count = 0;
        for block in blocks {
            let data_json = serde_json::to_string(&block.data)
                .map_err(|e| DatabaseError::Serialization(e.to_string()))?;

            tx.execute(
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
            count += 1;
        }
        
        tx.commit()?;
        info!(block_count = count, "Database: Saved blocks in batch");
        Ok(count)
    }

    /// Get a block by its index
    pub fn get_block_by_index(&self, index: u64) -> DbResult<Block> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT block_index, timestamp, data_json, prev_hash, hash, nonce 
             FROM blockchain WHERE block_index = ?"
        )?;

        let block_result = stmt.query_row([index], |row| {
            let idx: u64 = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            let data_json: String = row.get(2)?;
            let prev_hash: String = row.get(3)?;
            let hash: String = row.get(4)?;
            let nonce: u64 = row.get(5)?;

            let data: Vec<crate::etl::MarketData> = serde_json::from_str(&data_json)
                .map_err(|_e| rusqlite::Error::InvalidColumnType(2, "data_json".to_string(), rusqlite::types::Type::Text))?;

            Ok(Block {
                index: idx,
                timestamp,
                data,
                previous_hash: prev_hash,
                hash,
                nonce,
            })
        });

        match block_result {
            Ok(block) => Ok(block),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(DatabaseError::NotFound(format!("Block with index {} not found", index)))
            }
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    /// Get a block by its hash
    pub fn get_block_by_hash(&self, hash: &str) -> DbResult<Block> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT block_index, timestamp, data_json, prev_hash, hash, nonce 
             FROM blockchain WHERE hash = ?"
        )?;

        let block_result = stmt.query_row([hash], |row| {
            let idx: u64 = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            let data_json: String = row.get(2)?;
            let prev_hash: String = row.get(3)?;
            let hash: String = row.get(4)?;
            let nonce: u64 = row.get(5)?;

            let data: Vec<crate::etl::MarketData> = serde_json::from_str(&data_json)
                .map_err(|_e| rusqlite::Error::InvalidColumnType(2, "data_json".to_string(), rusqlite::types::Type::Text))?;

            Ok(Block {
                index: idx,
                timestamp,
                data,
                previous_hash: prev_hash,
                hash,
                nonce,
            })
        });

        match block_result {
            Ok(block) => Ok(block),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(DatabaseError::NotFound(format!("Block with hash {} not found", hash)))
            }
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    /// Get the latest block in the chain
    pub fn get_latest_block(&self) -> DbResult<Option<Block>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT block_index, timestamp, data_json, prev_hash, hash, nonce 
             FROM blockchain ORDER BY block_index DESC LIMIT 1"
        )?;

        let block_result = stmt.query_row([], |row| {
            let idx: u64 = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            let data_json: String = row.get(2)?;
            let prev_hash: String = row.get(3)?;
            let hash: String = row.get(4)?;
            let nonce: u64 = row.get(5)?;

            let data: Vec<crate::etl::MarketData> = serde_json::from_str(&data_json)
                .map_err(|_e| rusqlite::Error::InvalidColumnType(2, "data_json".to_string(), rusqlite::types::Type::Text))?;

            Ok(Block {
                index: idx,
                timestamp,
                data,
                previous_hash: prev_hash,
                hash,
                nonce,
            })
        });

        match block_result {
            Ok(block) => Ok(Some(block)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::Sqlite(e)),
        }
    }

    /// Query latest blocks and return them (instead of just printing)
    pub fn query_latest_blocks(&self, limit: u64) -> DbResult<Vec<Block>> {
        // Convert u64 to i64 for SQLite LIMIT clause (SQLite INTEGER is signed)
        // Cap at i64::MAX to avoid overflow
        let limit_i64 = limit.min(i64::MAX as u64) as i64;
        
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT block_index, timestamp, data_json, prev_hash, hash, nonce 
             FROM blockchain ORDER BY block_index DESC LIMIT ?"
        )?;

        let rows = stmt.query_map([limit_i64], |row| {
            let idx: u64 = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            let data_json: String = row.get(2)?;
            let prev_hash: String = row.get(3)?;
            let hash: String = row.get(4)?;
            let nonce: u64 = row.get(5)?;

            let data: Vec<crate::etl::MarketData> = serde_json::from_str(&data_json)
                .map_err(|_e| rusqlite::Error::InvalidColumnType(2, "data_json".to_string(), rusqlite::types::Type::Text))?;

            Ok(Block {
                index: idx,
                timestamp,
                data,
                previous_hash: prev_hash,
                hash,
                nonce,
            })
        })?;

        let mut blocks = Vec::new();
        for row in rows {
            blocks.push(row?);
        }
        Ok(blocks)
    }

    /// Print latest blocks (backward compatibility)
    pub fn print_latest_blocks(&self, limit: u64) -> DbResult<()> {
        let blocks = self.query_latest_blocks(limit)?;
        
        info!("Audit: Verifying latest blocks in DB");
        for block in blocks {
            let data_preview = serde_json::to_string(&block.data)
                .unwrap_or_else(|_| "Invalid JSON".to_string());
            debug!(
                block_index = block.index,
                hash_preview = &block.hash[0..8.min(block.hash.len())],
                data_preview = &data_preview[0..50.min(data_preview.len())],
                "Block details"
            );
        }
        Ok(())
    }

    /// Get the total number of blocks in the database
    pub fn get_block_count(&self) -> DbResult<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM blockchain", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Get blocks in a range (for pagination)
    pub fn get_blocks_range(&self, start_index: u64, end_index: u64) -> DbResult<Vec<Block>> {
        // Convert u64 to i64 for SQLite compatibility (SQLite INTEGER is signed)
        // This is safe as block indices will never exceed i64::MAX in practice
        let start_i64 = start_index as i64;
        let end_i64 = end_index as i64;
        
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT block_index, timestamp, data_json, prev_hash, hash, nonce 
             FROM blockchain WHERE block_index >= ? AND block_index <= ? 
             ORDER BY block_index ASC"
        )?;

        let rows = stmt.query_map(params![start_i64, end_i64], |row| {
            let idx: u64 = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            let data_json: String = row.get(2)?;
            let prev_hash: String = row.get(3)?;
            let hash: String = row.get(4)?;
            let nonce: u64 = row.get(5)?;

            let data: Vec<crate::etl::MarketData> = serde_json::from_str(&data_json)
                .map_err(|_e| rusqlite::Error::InvalidColumnType(2, "data_json".to_string(), rusqlite::types::Type::Text))?;

            Ok(Block {
                index: idx,
                timestamp,
                data,
                previous_hash: prev_hash,
                hash,
                nonce,
            })
        })?;

        let mut blocks = Vec::new();
        for row in rows {
            blocks.push(row?);
        }
        Ok(blocks)
    }

    /// Verify blockchain integrity by checking hash chain
    pub fn verify_chain(&self) -> DbResult<bool> {
        // Get all blocks without limit (use a large but safe i64 value)
        let limit = i64::MAX as u64;
        let blocks = self.query_latest_blocks(limit)?;
        
        if blocks.is_empty() {
            return Ok(true);
        }

        // Sort by index ascending
        let mut sorted_blocks = blocks;
        sorted_blocks.sort_by_key(|b| b.index);

        for i in 1..sorted_blocks.len() {
            let prev_block = &sorted_blocks[i - 1];
            let curr_block = &sorted_blocks[i];

            // Verify previous hash matches
            if curr_block.previous_hash != prev_block.hash {
                return Ok(false);
            }

            // Verify hash calculation
            let calculated_hash = curr_block.calculate_hash();
            if calculated_hash != curr_block.hash {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Delete a block by index (use with caution)
    pub fn delete_block(&self, index: u64) -> DbResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "DELETE FROM blockchain WHERE block_index = ?",
            [index],
        )?;
        
        Ok(rows_affected > 0)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> DbResult<DatabaseStats> {
        let conn = self.conn.lock().unwrap();
        
        let total_blocks: u64 = conn.query_row(
            "SELECT COUNT(*) FROM blockchain",
            [],
            |row| row.get(0)
        )?;

        let (min_index, max_index): (Option<u64>, Option<u64>) = conn.query_row(
            "SELECT MIN(block_index), MAX(block_index) FROM blockchain",
            [],
            |row| Ok((row.get(0)?, row.get(1)?))
        )?;

        let (min_timestamp, max_timestamp): (Option<i64>, Option<i64>) = conn.query_row(
            "SELECT MIN(timestamp), MAX(timestamp) FROM blockchain",
            [],
            |row| Ok((row.get(0)?, row.get(1)?))
        )?;

        Ok(DatabaseStats {
            total_blocks,
            min_index,
            max_index,
            min_timestamp,
            max_timestamp,
        })
    }
}

/// Database statistics structure
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_blocks: u64,
    pub min_index: Option<u64>,
    pub max_index: Option<u64>,
    pub min_timestamp: Option<i64>,
    pub max_timestamp: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::etl::{Block, MarketData};
    use std::fs;
    
    // Initialize logger for tests (only once)
    static INIT: std::sync::Once = std::sync::Once::new();
    
    fn init() {
        INIT.call_once(|| {
            // Suppress logs in tests unless RUST_LOG is explicitly set
            let _ = tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error"))
                )
                .with_test_writer()
                .try_init();
        });
    }

    fn create_test_block(index: u64, previous_hash: &str) -> Block {
        let mut block = Block {
            index,
            timestamp: 1234567890 + index as i64,
            data: vec![MarketData {
                asset: "BTC".to_string(),
                price: 50000.0 + index as f32,
                source: "Test".to_string(),
                timestamp: 1234567890 + index as i64,
            }],
            previous_hash: previous_hash.to_string(),
            hash: String::new(),
            nonce: 0,
        };
        block.calculate_hash_with_nonce();
        block
    }

    #[test]
    fn test_database_manager_new() {
        init();
        let test_db = "test_new.db";
        let result = DatabaseManager::new(test_db);
        assert!(result.is_ok());
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_database_init() {
        init();
        let test_db = "test_init.db";
        let db = DatabaseManager::new(test_db).unwrap();
        assert!(db.init().is_ok());
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_save_and_get_block_by_index() {
        init();
        let test_db = "test_get_by_index.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        let block = create_test_block(1, "0000_genesis");
        db.save_block(&block).unwrap();
        
        let retrieved = db.get_block_by_index(1).unwrap();
        assert_eq!(retrieved.index, 1);
        assert_eq!(retrieved.hash, block.hash);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_get_block_by_hash() {
        init();
        let test_db = "test_get_by_hash.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        let block = create_test_block(1, "0000_genesis");
        db.save_block(&block).unwrap();
        
        let retrieved = db.get_block_by_hash(&block.hash).unwrap();
        assert_eq!(retrieved.index, 1);
        assert_eq!(retrieved.hash, block.hash);
        
        // Test not found
        let result = db.get_block_by_hash("nonexistent_hash");
        assert!(result.is_err());
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_get_latest_block() {
        init();
        let test_db = "test_latest.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        // Empty database
        let latest = db.get_latest_block().unwrap();
        assert!(latest.is_none());
        
        // Add blocks
        let block1 = create_test_block(1, "0000_genesis");
        db.save_block(&block1).unwrap();
        
        let latest = db.get_latest_block().unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().index, 1);
        
        let block2 = create_test_block(2, &block1.hash);
        db.save_block(&block2).unwrap();
        
        let latest = db.get_latest_block().unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().index, 2);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_query_latest_blocks() {
        init();
        let test_db = "test_query_latest.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        let block1 = create_test_block(1, "0000_genesis");
        let block2 = create_test_block(2, &block1.hash);
        let block3 = create_test_block(3, &block2.hash);
        
        db.save_block(&block1).unwrap();
        db.save_block(&block2).unwrap();
        db.save_block(&block3).unwrap();
        
        let blocks = db.query_latest_blocks(2).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].index, 3); // Latest first
        assert_eq!(blocks[1].index, 2);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_get_blocks_range() {
        init();
        let test_db = "test_range.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        let mut prev_hash = "0000_genesis".to_string();
        for i in 1..=5 {
            let block = create_test_block(i, &prev_hash);
            prev_hash = block.hash.clone();
            db.save_block(&block).unwrap();
        }
        
        let blocks = db.get_blocks_range(2, 4).unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].index, 2);
        assert_eq!(blocks[1].index, 3);
        assert_eq!(blocks[2].index, 4);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_save_blocks_batch() {
        init();
        let test_db = "test_batch.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        let mut blocks = Vec::new();
        let mut prev_hash = "0000_genesis".to_string();
        for i in 1..=3 {
            let block = create_test_block(i, &prev_hash);
            prev_hash = block.hash.clone();
            blocks.push(block);
        }
        
        let saved = db.save_blocks(&blocks).unwrap();
        assert_eq!(saved, 3);
        
        let count = db.get_block_count().unwrap();
        assert_eq!(count, 3);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_verify_chain_valid() {
        init();
        let test_db = "test_verify_valid.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        let mut prev_hash = "0000_genesis".to_string();
        for i in 1..=3 {
            let block = create_test_block(i, &prev_hash);
            prev_hash = block.hash.clone();
            db.save_block(&block).unwrap();
        }
        
        let is_valid = db.verify_chain().unwrap();
        assert!(is_valid);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_verify_chain_invalid() {
        init();
        let test_db = "test_verify_invalid.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        let block1 = create_test_block(1, "0000_genesis");
        db.save_block(&block1).unwrap();
        
        // Create block with wrong previous hash
        let mut block2 = create_test_block(2, "wrong_hash");
        db.save_block(&block2).unwrap();
        
        let is_valid = db.verify_chain().unwrap();
        assert!(!is_valid);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_delete_block() {
        init();
        let test_db = "test_delete.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        let block = create_test_block(1, "0000_genesis");
        db.save_block(&block).unwrap();
        
        assert_eq!(db.get_block_count().unwrap(), 1);
        
        let deleted = db.delete_block(1).unwrap();
        assert!(deleted);
        
        assert_eq!(db.get_block_count().unwrap(), 0);
        
        let deleted = db.delete_block(999).unwrap();
        assert!(!deleted);
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_get_stats() {
        init();
        let test_db = "test_stats.db";
        fs::remove_file(test_db).ok();
        
        let db = DatabaseManager::new(test_db).unwrap();
        db.init().unwrap();
        
        // Empty database
        let stats = db.get_stats().unwrap();
        assert_eq!(stats.total_blocks, 0);
        assert!(stats.min_index.is_none());
        assert!(stats.max_index.is_none());
        
        // Add blocks
        let mut prev_hash = "0000_genesis".to_string();
        for i in 1..=3 {
            let block = create_test_block(i, &prev_hash);
            prev_hash = block.hash.clone();
            db.save_block(&block).unwrap();
        }
        
        let stats = db.get_stats().unwrap();
        assert_eq!(stats.total_blocks, 3);
        assert_eq!(stats.min_index, Some(1));
        assert_eq!(stats.max_index, Some(3));
        assert!(stats.min_timestamp.is_some());
        assert!(stats.max_timestamp.is_some());
        
        fs::remove_file(test_db).ok();
    }

    #[test]
    fn test_database_error_display() {
        init();
        let error = DatabaseError::NotFound("test".to_string());
        let error_str = format!("{}", error);
        assert!(error_str.contains("Not found"));
        assert!(error_str.contains("test"));
    }
}
