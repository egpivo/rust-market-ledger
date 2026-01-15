pub mod extract;
pub mod transform;
pub mod load;
pub mod validator;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarketData {
    pub asset: String,
    pub price: f32,
    pub source: String,
    pub timestamp: i64,
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
