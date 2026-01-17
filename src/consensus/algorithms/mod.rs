//! Consensus algorithm implementations
//!
//! **Note**: These are conceptual implementations designed for educational and demonstration purposes.
//! They are simplified versions that capture core algorithm characteristics and trade-offs but are
//! not production-ready. These implementations use simulated network communication and simplified
//! state management for comparative analysis and understanding consensus algorithm concepts.

pub mod eventual;
pub mod flexible_paxos;
pub mod gossip;
pub mod pbft;
pub mod quorumless;

// Re-export PBFT types for backward compatibility
pub use pbft::{MessageType, PBFTManager, PBFTMessage};
