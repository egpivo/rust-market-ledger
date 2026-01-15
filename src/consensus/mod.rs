//! Consensus algorithm abstraction and implementations
//! 
//! This module provides a trait-based consensus system that allows
//! plugging in different consensus algorithms, including those that
//! don't require majority voting.
//!
//! ## Structure
//! - `traits.rs` - Consensus algorithm trait definition
//! - `types.rs` - Common types and data structures
//! - `algorithms/` - Individual consensus algorithm implementations
//!   - `pbft.rs` - PBFT (requires majority voting)
//!   - `gossip.rs` - Gossip protocol (no majority voting)
//!   - `eventual.rs` - Eventual consistency (no majority voting)
//!   - `quorumless.rs` - Weighted voting (no majority voting)
//! - `examples.rs` - Usage examples
//! - `tests.rs` - Unit tests

// Re-export public API
pub use traits::ConsensusAlgorithm;
pub use types::{ConsensusMessage, ConsensusResult, ConsensusRequirements};

// Algorithm implementations
pub mod algorithms;

// Examples
pub mod examples;

// Tests
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

// Internal modules
mod traits;
mod types;
