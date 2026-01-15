use crate::etl::validator::Validator;
use std::error::Error;

pub struct Transformer {
    validator: Validator,
    deduplication_window_seconds: i64,
}

pub struct TransformResult {
    pub asset: String,
    pub price: f32,
    pub source: String,
    pub timestamp: i64,
    pub is_deduplicated: bool,
}

impl Transformer {
    pub fn new() -> Self {
        Transformer {
            validator: Validator::new(),
            deduplication_window_seconds: 60,
        }
    }

    pub fn with_validator(mut self, validator: Validator) -> Self {
        self.validator = validator;
        self
    }

    pub fn with_deduplication_window(mut self, seconds: i64) -> Self {
        self.deduplication_window_seconds = seconds;
        self
    }

    pub fn transform(
        &self,
        price: f32,
        timestamp: i64,
        source: String,
        last_timestamp: Option<i64>,
    ) -> Result<TransformResult, Box<dyn Error>> {
        self.validator.validate_price(price)?;
        self.validator.validate_timestamp(timestamp)?;
        self.validator.validate_source(&source)?;

        let is_deduplicated = if let Some(last_ts) = last_timestamp {
            (timestamp - last_ts).abs() < self.deduplication_window_seconds
        } else {
            false
        };

        Ok(TransformResult {
            asset: "BTC".to_string(),
            price,
            source,
            timestamp,
            is_deduplicated,
        })
    }

    pub fn normalize_price(&self, price: f32) -> f32 {
        (price * 100.0).round() / 100.0
    }

    pub fn deduplication_window_seconds(&self) -> i64 {
        self.deduplication_window_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::etl::validator::Validator;

    #[test]
    fn test_transformer_creation() {
        let transformer = Transformer::new();
        assert_eq!(transformer.deduplication_window_seconds(), 60);
    }

    #[test]
    fn test_transformer_with_validator() {
        use chrono::Utc;
        let validator = Validator::new()
            .with_price_range(0.0, 100000.0)
            .with_timestamp_drift(86400); // 24 hours
        let transformer = Transformer::new().with_validator(validator);
        let timestamp = Utc::now().timestamp();
        assert!(transformer.transform(50000.0, timestamp, "Test".to_string(), None).is_ok());
    }

    #[test]
    fn test_transformer_with_deduplication_window() {
        let transformer = Transformer::new().with_deduplication_window(30);
        assert_eq!(transformer.deduplication_window_seconds(), 30);
    }

    #[test]
    fn test_transform_valid_data() {
        use chrono::Utc;
        let transformer = Transformer::new();
        let timestamp = Utc::now().timestamp();
        let result = transformer.transform(
            50000.0,
            timestamp,
            "CoinGecko".to_string(),
            None,
        ).unwrap();

        assert_eq!(result.asset, "BTC");
        assert_eq!(result.price, 50000.0);
        assert_eq!(result.source, "CoinGecko");
        assert_eq!(result.timestamp, timestamp);
        assert!(!result.is_deduplicated);
    }

    #[test]
    fn test_transform_invalid_price() {
        let transformer = Transformer::new();
        let result = transformer.transform(
            -100.0,
            1234567890,
            "Test".to_string(),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_transform_invalid_timestamp() {
        let transformer = Transformer::new();
        let result = transformer.transform(
            50000.0,
            -1,
            "Test".to_string(),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_transform_invalid_source() {
        let transformer = Transformer::new();
        let result = transformer.transform(
            50000.0,
            1234567890,
            "".to_string(),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_transform_deduplication_detected() {
        use chrono::Utc;
        let validator = Validator::new().with_timestamp_drift(86400); // 24 hours
        let transformer = Transformer::new()
            .with_validator(validator)
            .with_deduplication_window(60);
        let timestamp = Utc::now().timestamp();
        
        // First transform - no deduplication
        let result1 = transformer.transform(
            50000.0,
            timestamp,
            "Test".to_string(),
            None,
        ).unwrap();
        assert!(!result1.is_deduplicated);

        // Second transform within window - should be deduplicated
        let result2 = transformer.transform(
            50100.0,
            timestamp + 30, // Within 60 second window
            "Test".to_string(),
            Some(timestamp),
        ).unwrap();
        assert!(result2.is_deduplicated);
    }

    #[test]
    fn test_transform_deduplication_not_detected() {
        use chrono::Utc;
        let validator = Validator::new().with_timestamp_drift(86400); // 24 hours
        let transformer = Transformer::new()
            .with_validator(validator)
            .with_deduplication_window(60);
        let timestamp = Utc::now().timestamp();
        
        // Transform outside window - should not be deduplicated
        let result = transformer.transform(
            50000.0,
            timestamp + 120, // Outside 60 second window
            "Test".to_string(),
            Some(timestamp),
        ).unwrap();
        assert!(!result.is_deduplicated);
    }

    #[test]
    fn test_normalize_price() {
        let transformer = Transformer::new();
        
        assert_eq!(transformer.normalize_price(50000.123), 50000.12);
        assert_eq!(transformer.normalize_price(50000.456), 50000.46);
        assert_eq!(transformer.normalize_price(50000.0), 50000.0);
        assert_eq!(transformer.normalize_price(50000.999), 50001.0);
    }

    #[test]
    fn test_transform_result_fields() {
        use chrono::Utc;
        let transformer = Transformer::new();
        let timestamp = Utc::now().timestamp();
        let result = transformer.transform(
            50000.0,
            timestamp,
            "TestSource".to_string(),
            None,
        ).unwrap();

        assert_eq!(result.asset, "BTC");
        assert_eq!(result.price, 50000.0);
        assert_eq!(result.source, "TestSource");
        assert_eq!(result.timestamp, timestamp);
        assert!(!result.is_deduplicated);
    }
}
