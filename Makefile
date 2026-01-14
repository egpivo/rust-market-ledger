.PHONY: help build test run clean start stop

help:
	@echo "Available targets:"
	@echo "  build       - Build the project"
	@echo "  test        - Run tests"
	@echo "  run         - Run single node (node_id=0, port=8000)"
	@echo "  start       - Start all 4 nodes"
	@echo "  stop        - Stop all nodes"
	@echo "  clean       - Clean build artifacts and databases"
	@echo "  check       - Check code without building"

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

run-node:
	@if [ -z "$(id)" ]; then \
		echo "Usage: make run-node id=0"; \
		exit 1; \
	fi
	cargo run -- $(id) $$((8000 + $(id)))

start:
	@bash scripts/start_nodes.sh

stop:
	@bash scripts/stop_nodes.sh

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
