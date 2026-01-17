//! Tests for consensus algorithms

#[cfg(test)]
mod consensus_tests {
    use crate::consensus::algorithms::*;
    use crate::consensus::*;
    use crate::etl::{Block, MarketData};
    use std::sync::Arc;
    use tokio::time::Duration;

    // Initialize logger for tests (only once)
    static INIT: std::sync::Once = std::sync::Once::new();

    fn init() {
        INIT.call_once(|| {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error")),
                )
                .with_test_writer()
                .try_init();
        });
    }

    fn create_test_block(index: u64) -> Block {
        let mut block = Block {
            index,
            timestamp: chrono::Utc::now().timestamp(),
            data: vec![MarketData {
                asset: "BTC".to_string(),
                price: 50000.0 + index as f32,
                source: "Test".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
            }],
            previous_hash: if index == 1 {
                "0000_genesis".to_string()
            } else {
                format!("hash_{}", index - 1)
            },
            hash: String::new(),
            nonce: 0,
        };
        block.calculate_hash_with_nonce();
        block
    }

    #[tokio::test]
    async fn test_gossip_consensus() {
        init();
        let consensus = Arc::new(gossip::GossipConsensus::new(0, 1, 2));
        let block = create_test_block(1);

        let result = consensus.propose(&block).await.unwrap();

        match result {
            ConsensusResult::Committed(_) => {
                assert!(consensus.is_committed(1));
            }
            _ => panic!("Expected committed result"),
        }
    }

    #[tokio::test]
    async fn test_eventual_consensus() {
        init();
        let consensus = Arc::new(eventual::EventualConsensus::new(0, 50, 1));
        let block = create_test_block(1);

        let start = std::time::Instant::now();
        let result = consensus.propose(&block).await.unwrap();
        let elapsed = start.elapsed();

        match result {
            ConsensusResult::Committed(_) => {
                assert!(elapsed >= Duration::from_millis(50));
                assert!(consensus.is_committed(1));
            }
            _ => panic!("Expected committed result"),
        }
    }

    #[tokio::test]
    async fn test_quorumless_consensus() {
        init();
        let consensus = Arc::new(quorumless::QuorumlessConsensus::new(0, 3.0));

        consensus.set_node_weight(0, 2.0);
        consensus.set_node_weight(1, 2.0);

        let block = create_test_block(1);
        let result = consensus.propose(&block).await.unwrap();

        match result {
            ConsensusResult::Pending => {
                // Expected - need more votes
            }
            _ => panic!("Expected pending result, got {:?}", result),
        }
    }

    #[test]
    fn test_consensus_requirements() {
        init();
        let gossip = gossip::GossipConsensus::new(0, 3, 2);
        let req = gossip.requirements();

        assert!(!req.requires_majority);
        assert_eq!(req.min_nodes, None);

        let eventual = eventual::EventualConsensus::new(0, 1000, 2);
        let req = eventual.requirements();

        assert!(!req.requires_majority);
        assert_eq!(req.min_nodes, None);
    }

    #[test]
    fn test_consensus_names() {
        init();
        let gossip = gossip::GossipConsensus::new(0, 3, 2);
        assert_eq!(gossip.name(), "Gossip Protocol");

        let eventual = eventual::EventualConsensus::new(0, 1000, 2);
        assert_eq!(eventual.name(), "Eventual Consistency");

        let quorumless = quorumless::QuorumlessConsensus::new(0, 5.0);
        assert_eq!(quorumless.name(), "Quorum-less (Weighted)");
    }
}
