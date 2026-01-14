#!/bin/bash

# Stop all running blockchain nodes

echo "🛑 Stopping all blockchain nodes..."

if [ -f node_0.pid ]; then
    for pidfile in node_*.pid; do
        if [ -f "$pidfile" ]; then
            pid=$(cat "$pidfile")
            if kill -0 "$pid" 2>/dev/null; then
                echo "Stopping process $pid..."
                kill "$pid"
            fi
            rm -f "$pidfile"
        fi
    done
fi

# Also try pkill as backup
pkill -f "rust-market-ledger" 2>/dev/null || true

echo "✅ All nodes stopped."
echo "📋 Logs are preserved in node_*.log files"
