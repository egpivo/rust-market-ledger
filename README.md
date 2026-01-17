# rust-market-ledger

[![CI](https://github.com/egpivo/rust-market-ledger/actions/workflows/ci.yml/badge.svg)](https://github.com/egpivo/rust-market-ledger/actions/workflows/ci.yml)

A demonstration repository for comparing different distributed consensus algorithms in a market ledger context.

## Overview

This project implements a market ledger system with pluggable consensus algorithms, demonstrating various consensus mechanisms (PBFT, Gossip, Eventual Consistency, Quorum-less, Flexible Paxos) and their trade-offs.

**Important Note**: The consensus algorithm implementations in this repository are **conceptual implementations** designed for educational and demonstration purposes. They are simplified versions that capture the core characteristics and trade-offs of each algorithm but are not production-ready implementations. These implementations are intended for:
- Understanding consensus algorithm concepts and trade-offs
- Comparing performance characteristics in a controlled simulation environment
- Educational demonstrations of the blockchain trilemma

For production use, please refer to established consensus libraries and implementations.

## Features

- **ETL Pipeline**: Extract, Transform, Load pipeline for market data
- **Pluggable Consensus**: Support for multiple consensus algorithms
- **Consensus Comparison**: Framework for comparing consensus strategies
- **Examples**: Educational examples demonstrating different consensus approaches

## Quick Start

### Prerequisites

- Rust 1.70+ ([rustup.rs](https://rustup.rs))

### Installation

```bash
git clone https://github.com/egpivo/rust-market-ledger.git
cd rust-market-ledger
cargo build
```

### Run Examples

See [examples/README.md](examples/README.md) for detailed examples and comparison scenarios.

## Examples

For comprehensive examples and consensus comparison experiments, see:
- **[examples/README.md](examples/README.md)** - Complete guide to all examples

Quick run:
```bash
cargo run --example trilemma_comparison
```

## CI Status

All code is automatically checked with:
- `cargo fmt` - Code formatting
- `cargo clippy` - Linting
- `cargo test` - Unit tests
- `cargo build` - Compilation check

## License

See [LICENSE](LICENSE) file for details.
