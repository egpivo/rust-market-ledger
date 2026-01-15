use crate::etl::validator::Validator;
use chrono::prelude::*;
use reqwest::Client;
use std::error::Error;
use std::time::Duration;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct CoinGeckoResponse {
    bitcoin: PriceDetail,
}

#[derive(Deserialize, Debug)]
struct PriceDetail {
    usd: f32,
}

pub struct Extractor {
    client: Client,
    validator: Validator,
    max_retries: u32,
}

pub struct ExtractResult {
    pub price: f32,
    pub timestamp: i64,
    pub source: String,
}

impl Extractor {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let client = Client::builder()
            .user_agent("rust-market-ledger/0.1.0")
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Extractor {
            client,
            validator: Validator::new(),
            max_retries: 3,
        })
    }

    pub fn with_validator(mut self, validator: Validator) -> Self {
        self.validator = validator;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub async fn extract_from_api(&self) -> Result<ExtractResult, Box<dyn Error>> {
        let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd";
        let mut last_error = None;

        for attempt in 1..=self.max_retries {
            match self.client.get(url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        last_error = Some(format!("HTTP status: {}", status));
                        if status == 429 || status == 403 {
                            let delay_ms = 1000 * attempt as u64;
                            if attempt < self.max_retries {
                                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                                continue;
                            }
                        } else if attempt < self.max_retries {
                            tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                            continue;
                        }
                        return Err(format!("API returned status: {}", status).into());
                    }

                    match response.json::<CoinGeckoResponse>().await {
                        Ok(resp) => {
                            let price = resp.bitcoin.usd;
                            let timestamp = Utc::now().timestamp();

                            self.validator.validate_price(price)?;
                            self.validator.validate_timestamp(timestamp)?;

                            return Ok(ExtractResult {
                                price,
                                timestamp,
                                source: "CoinGecko".to_string(),
                            });
                        }
                        Err(e) => {
                            last_error = Some(format!("JSON decode error: {}", e));
                            if attempt < self.max_retries {
                                tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(format!("Request error: {}", e));
                    if attempt < self.max_retries {
                        tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                        continue;
                    }
                }
            }
        }

        Err(format!(
            "Failed after {} attempts. Last error: {}",
            self.max_retries,
            last_error.unwrap_or_default()
        )
        .into())
    }

    pub async fn extract_offline(&self) -> Result<ExtractResult, Box<dyn Error>> {
        let timestamp = Utc::now().timestamp();
        let base_price = 50000.0;
        let variation = (timestamp % 1000) as f32 / 10.0;
        let price = base_price + variation;

        self.validator.validate_price(price)?;
        self.validator.validate_timestamp(timestamp)?;

        Ok(ExtractResult {
            price,
            timestamp,
            source: "MockData".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extractor_creation() {
        let extractor = Extractor::new();
        assert!(extractor.is_ok());
    }

    #[tokio::test]
    async fn test_extractor_with_max_retries() {
        let extractor = Extractor::new().unwrap();
        let extractor = extractor.with_max_retries(5);
        // We can't directly access max_retries, but we can test the builder pattern works
        assert!(extractor.extract_offline().await.is_ok());
    }

    #[tokio::test]
    async fn test_extractor_with_validator() {
        let validator = Validator::new().with_price_range(0.0, 100000.0);
        let extractor = Extractor::new().unwrap();
        let extractor = extractor.with_validator(validator);
        assert!(extractor.extract_offline().await.is_ok());
    }

    #[tokio::test]
    async fn test_extract_offline() {
        let extractor = Extractor::new().unwrap();
        let result = extractor.extract_offline().await;
        
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.source, "MockData");
        assert!(data.price >= 50000.0);
        assert!(data.price < 50100.0); // base_price + max variation
        assert!(data.timestamp > 0);
    }

    #[tokio::test]
    async fn test_extract_offline_validation() {
        let validator = Validator::new().with_price_range(0.0, 100.0);
        let extractor = Extractor::new()
            .unwrap()
            .with_validator(validator);
        
        // Offline extraction generates prices around 50000, which exceeds max of 100
        let result = extractor.extract_offline().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_result_fields() {
        let extractor = Extractor::new().unwrap();
        let result = extractor.extract_offline().await.unwrap();
        
        assert!(!result.source.is_empty());
        assert!(result.price > 0.0);
        assert!(result.timestamp > 0);
    }
}
