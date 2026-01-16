//! Consensus algorithm implementations

// PBFT implementation (internal)
mod pbft_impl;
// PBFT consensus adapter (implements ConsensusAlgorithm trait)
pub mod pbft;

pub mod gossip;
pub mod eventual;
pub mod quorumless;

// Re-export PBFT types for backward compatibility
pub use pbft_impl::{PBFTManager, PBFTMessage, MessageType};
