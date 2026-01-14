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

info "Stopping all blockchain nodes..."

stopped_count=0

if [ -f node_0.pid ]; then
    for pidfile in node_*.pid; do
        if [ -f "$pidfile" ]; then
            pid=$(cat "$pidfile")
            if kill -0 "$pid" 2>/dev/null; then
                info "Stopping process $pid..."
                if kill "$pid" 2>/dev/null; then
                    ((stopped_count++))
                fi
            fi
            rm -f "$pidfile"
        fi
    done
fi

pkill -f "rust-market-ledger" 2>/dev/null || true

if [ $stopped_count -gt 0 ]; then
    success "Stopped $stopped_count node(s)"
else
    warning "No running nodes found"
fi

info "Logs are preserved in node_*.log files"

exit $SUCCESS_EXITCODE
