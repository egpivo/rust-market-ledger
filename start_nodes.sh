#!/bin/bash

# Multi-node PBFT Consensus Startup Script
# This script starts 4 blockchain nodes for testing PBFT consensus

echo "🚀 Starting 4 PBFT Blockchain Nodes..."
echo ""

# Function to start a node in background
start_node() {
    local node_id=$1
    local port=$((8000 + node_id))
    echo "Starting Node $node_id on port $port..."
    cargo run --release -- $node_id $port > "node_${node_id}.log" 2>&1 &
    echo $! > "node_${node_id}.pid"
    echo "✅ Node $node_id started (PID: $(cat node_${node_id}.pid))"
    sleep 1
}

# Clean up old processes and logs
echo "🧹 Cleaning up old processes..."
pkill -f "rust-market-ledger" 2>/dev/null || true
rm -f node_*.pid node_*.log
sleep 1

# Start all nodes
for i in {0..3}; do
    start_node $i
done

echo ""
echo "📡 All nodes started!"
echo "📋 Node addresses:"
echo "   Node 0: http://127.0.0.1:8000"
echo "   Node 1: http://127.0.0.1:8001"
echo "   Node 2: http://127.0.0.1:8002"
echo "   Node 3: http://127.0.0.1:8003"
echo ""
echo "📊 Monitor logs with: tail -f node_*.log"
echo "🛑 Stop all nodes with: ./stop_nodes.sh"
echo ""
