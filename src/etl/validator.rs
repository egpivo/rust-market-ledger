use chrono::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub reason: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Validation error in {}: {}", self.field, self.reason)
    }
}

impl std::error::Error for ValidationError {}

pub struct Validator {
    min_price: f32,
    max_price: f32,
    max_timestamp_drift_seconds: i64,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    pub fn new() -> Self {
        Validator {
            min_price: 0.0,
            max_price: 1_000_000.0,
            max_timestamp_drift_seconds: 3600,
        }
    }

    pub fn with_price_range(mut self, min: f32, max: f32) -> Self {
        self.min_price = min;
        self.max_price = max;
        self
    }

    pub fn with_timestamp_drift(mut self, seconds: i64) -> Self {
        self.max_timestamp_drift_seconds = seconds;
        self
    }

    pub fn validate_price(&self, price: f32) -> Result<(), ValidationError> {
        if price < self.min_price {
            return Err(ValidationError {
                field: "price".to_string(),
                reason: format!("Price {} is below minimum {}", price, self.min_price),
            });
        }

        if price > self.max_price {
            return Err(ValidationError {
                field: "price".to_string(),
                reason: format!("Price {} exceeds maximum {}", price, self.max_price),
            });
        }

        if !price.is_finite() {
            return Err(ValidationError {
                field: "price".to_string(),
                reason: format!("Price {} is not finite (NaN or Infinity)", price),
            });
        }

        Ok(())
    }

    pub fn validate_timestamp(&self, timestamp: i64) -> Result<(), ValidationError> {
        let now = Utc::now().timestamp();
        let drift = (timestamp - now).abs();

        if drift > self.max_timestamp_drift_seconds {
            return Err(ValidationError {
                field: "timestamp".to_string(),
                reason: format!(
                    "Timestamp {} drifts {} seconds from current time (max: {})",
                    timestamp, drift, self.max_timestamp_drift_seconds
                ),
            });
        }

        if timestamp < 0 {
            return Err(ValidationError {
                field: "timestamp".to_string(),
                reason: "Timestamp cannot be negative".to_string(),
            });
        }

        Ok(())
    }

    pub fn validate_asset_symbol(&self, symbol: &str) -> Result<(), ValidationError> {
        if symbol.is_empty() {
            return Err(ValidationError {
                field: "asset".to_string(),
                reason: "Asset symbol cannot be empty".to_string(),
            });
        }

        if symbol.len() > 10 {
            return Err(ValidationError {
                field: "asset".to_string(),
                reason: format!("Asset symbol '{}' exceeds maximum length of 10", symbol),
            });
        }

        Ok(())
    }

    pub fn validate_source(&self, source: &str) -> Result<(), ValidationError> {
        if source.is_empty() {
            return Err(ValidationError {
                field: "source".to_string(),
                reason: "Source cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_price_positive() {
        let validator = Validator::new();
        assert!(validator.validate_price(50000.0).is_ok());
    }

    #[test]
    fn test_validate_price_negative() {
        let validator = Validator::new();
        assert!(validator.validate_price(-100.0).is_err());
    }

    #[test]
    fn test_validate_price_nan() {
        let validator = Validator::new();
        assert!(validator.validate_price(f32::NAN).is_err());
    }

    #[test]
    fn test_validate_price_infinity() {
        let validator = Validator::new();
        assert!(validator.validate_price(f32::INFINITY).is_err());
    }

    #[test]
    fn test_validate_timestamp_valid() {
        let validator = Validator::new();
        let timestamp = Utc::now().timestamp();
        assert!(validator.validate_timestamp(timestamp).is_ok());
    }

    #[test]
    fn test_validate_timestamp_negative() {
        let validator = Validator::new();
        assert!(validator.validate_timestamp(-1).is_err());
    }

    #[test]
    fn test_validate_asset_symbol() {
        let validator = Validator::new();
        assert!(validator.validate_asset_symbol("BTC").is_ok());
        assert!(validator.validate_asset_symbol("").is_err());
    }
}
