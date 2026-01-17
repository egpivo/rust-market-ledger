# Consensus Comparison Examples

## Comparison Experiments

These three examples demonstrate different consensus strategies while keeping the ETL + block generation flow unchanged.

### Design Philosophy

- **Keep ETL + Block flow unchanged** - Only replace consensus module
- **Abstract consensus strategy** - Use `ConsensusStrategy` trait for unified interface
- **Easy comparison** - Same block can go through different consensus strategies
- **Minimal intrusion** - No need to modify existing ETL/Block generation code

### Example A: No-Consensus (Single Node Direct Commit)

**File**: `examples/no_consensus_example.rs`

**Purpose**:
- Understand consensus necessity vs cost
- Compare latency and safety differences with PBFT

**Features**:
- Zero latency - immediate commit
- No safety guarantees
- Suitable for single-node scenarios

**Run**:
```bash
cargo run --example no_consensus_example
```

### Example B: Simple Majority Vote

**File**: `examples/simple_majority_example.rs`

**Purpose**:
- Understand simple majority voting mechanism
- Compare BFT advantages with PBFT
- Show non-Byzantine vs Byzantine fault tolerance differences

**Features**:
- Simple majority voting (n/2 + 1)
- Cannot tolerate Byzantine faults
- Simpler than PBFT but lower safety

**Run**:
```bash
cargo run --example simple_majority_example
```

### Example C: PBFT (Baseline)

**File**: `examples/pbft_baseline_example.rs`

**Purpose**:
- Show complete PBFT consensus flow
- Serve as baseline for comparing with other consensus algorithms
- Understand complete BFT consensus flow

**Features**:
- Three-phase protocol: Pre-Prepare, Prepare, Commit
- Byzantine fault tolerance: can tolerate f malicious nodes (3f+1 total)
- Safety guarantee: consensus even with malicious nodes

**Run**:
```bash
cargo run --example pbft_baseline_example
```

### Run All Examples

**File**: `examples/run_all_comparisons.rs`

Run all three examples in sequence for comparison:

```bash
cargo run --example run_all_comparisons
```

## Trilemma Comparison Experiment

**File**: `examples/trilemma_comparison.rs`

**Purpose**:
- Combine performance metrics with qualitative trilemma scores
- Report runtime and standard deviation for credibility
- Provide a reproducible baseline for Medium articles

**Run**:
```bash
cargo run --example trilemma_comparison
```

## Comparison Summary

| Strategy | Latency | Safety | BFT | Complexity |
|----------|---------|--------|-----|------------|
| No-Consensus | Zero | None | No | Very Low |
| Simple Majority | Low | Medium | No | Low |
| PBFT | High | High | Yes | High |

## Key Insights

1. **Consensus Necessity**: No-Consensus shows the cost of no consensus (no safety)
2. **BFT Importance**: Simple Majority vs PBFT shows the value of Byzantine fault tolerance
3. **Trade-offs**: Latency vs Safety vs Complexity trade-offs
