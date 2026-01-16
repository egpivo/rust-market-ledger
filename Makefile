.PHONY: help build test run clean start stop install-env query

help:
	@echo "Available targets:"
	@echo "  install-env - Install Rust and Cargo (macOS/Linux)"
	@echo "  build       - Build the project"
	@echo "  test        - Run tests"
	@echo "  run         - Run single node (node_id=0, port=8000)"
	@echo "  start       - Start all 4 nodes"
	@echo "  start-offline - Start all 4 nodes in offline mode"
	@echo "  test-offline - Test single node in offline mode"
	@echo "  stop        - Stop all nodes"
	@echo "  clean       - Clean build artifacts and databases"
	@echo "  check       - Check code without building"
	@echo "  query       - Query database (usage: make query id=0)"

build:
	cargo build --release

test:
	cargo test

test-verbose:
	cargo test -- --nocapture

check:
	cargo check

run:
	cargo run -- 0 8000

run-offline:
	cargo run -- 0 8000 --offline

run-node:
	@if [ -z "$(id)" ]; then \
		echo "Usage: make run-node id=0"; \
		exit 1; \
	fi
	cargo run -- $(id) $$((8000 + $(id)))

start:
	@bash scripts/start_nodes.sh

start-offline:
	@bash scripts/start_nodes.sh --offline

test-offline:
	@bash scripts/test_offline.sh

stop:
	@bash scripts/stop_nodes.sh

query:
	@if [ -z "$(id)" ]; then \
		echo "Usage: make query id=0"; \
		exit 1; \
	fi
	@bash scripts/query_db.sh $(id)

clean:
	cargo clean
	rm -f blockchain_node_*.db
	rm -f node_*.log node_*.pid
	rm -f test_*.db

clean-all: clean
	rm -rf target/

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

install-env:
	@bash scripts/install_rust.sh
	@echo "Development environment setup complete!"
	@echo ""
	@if command -v cargo &> /dev/null; then \
		echo "Rust/Cargo installed: $$(cargo --version)"; \
	else \
		echo "Please restart your shell or run: source $$HOME/.cargo/env"; \
	fi
	@echo ""
	@echo "Next steps:"
	@echo "  1. If cargo is not available, run: source $$HOME/.cargo/env"
	@echo "  2. Build the project: make build"
	@echo "  3. Run tests: make test"
