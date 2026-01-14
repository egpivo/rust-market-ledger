#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/color_map.sh"
source "${SCRIPT_DIR}/exit_code.sh"

error() {
    echo -e "${FG_RED}Error:${FG_RESET} $1" >&2
    exit $ERROR_EXITCODE
}

success() {
    echo -e "${FG_GREEN}${1}${FG_RESET}"
}

info() {
    echo -e "${FG_BLUE}${1}${FG_RESET}"
}

warning() {
    echo -e "${FG_YELLOW}Warning:${FG_RESET} $1"
}

if ! command -v cargo &> /dev/null; then
    error "cargo not found. Please install Rust first: make install-rust"
fi

info "Starting 4 PBFT Blockchain Nodes..."
echo ""

start_node() {
    local node_id=$1
    local port=$((8000 + node_id))
    local offline_mode=${2:-""}
    
    info "Starting Node $node_id on port $port..."
    if [ -n "$offline_mode" ]; then
        info "Using offline mode (mock data)"
    fi
    
    if [ -n "$offline_mode" ]; then
        cargo run --release -- $node_id $port --offline > "node_${node_id}.log" 2>&1 &
    else
        cargo run --release -- $node_id $port > "node_${node_id}.log" 2>&1 &
    fi
    local pid=$!
    echo $pid > "node_${node_id}.pid"
    
    sleep 0.5
    
    if ! kill -0 $pid 2>/dev/null; then
        error "Node $node_id failed to start (check node_${node_id}.log for details)"
    fi
    
    success "Node $node_id started (PID: $pid)"
    sleep 1
}

info "Cleaning up old processes..."
pkill -f "rust-market-ledger" 2>/dev/null || true
rm -f node_*.pid node_*.log
sleep 1

OFFLINE_MODE=""
if [ "$1" = "--offline" ] || [ "$1" = "-o" ]; then
    OFFLINE_MODE="--offline"
    info "Starting nodes in OFFLINE mode (using mock data)"
fi

for i in {0..3}; do
    start_node $i "$OFFLINE_MODE"
done

echo ""
success "All nodes started!"
info "Node addresses:"
echo "   Node 0: http://127.0.0.1:8000"
echo "   Node 1: http://127.0.0.1:8001"
echo "   Node 2: http://127.0.0.1:8002"
echo "   Node 3: http://127.0.0.1:8003"
echo ""
info "Monitor logs with: tail -f node_*.log"
info "Stop all nodes with: make stop"
echo ""

exit $SUCCESS_EXITCODE
