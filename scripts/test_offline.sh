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

if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    else
        error "cargo not found. Please install Rust first: make install-env"
    fi
fi

info "Testing offline mode (single node)..."
echo ""

DB_FILE="blockchain_node_0.db"
if [ -f "$DB_FILE" ]; then
    info "Removing old database: $DB_FILE"
    rm -f "$DB_FILE"
fi

info "Running node 0 in offline mode..."
echo ""

cargo run --release -- 0 8000 --offline 2>&1 | tee test_offline.log

echo ""
echo ""

if [ -f "$DB_FILE" ]; then
    info "Checking database results..."
    echo ""
    
    TOTAL=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM blockchain;" 2>/dev/null || echo "0")
    
    if [ "$TOTAL" -gt 0 ]; then
        success "Found $TOTAL block(s) in database!"
        echo ""
        info "Latest blocks:"
        sqlite3 -header -column "$DB_FILE" "SELECT block_index, substr(hash, 1, 16) as hash_preview, substr(data_json, 1, 60) as data_preview FROM blockchain ORDER BY block_index DESC LIMIT 5;" 2>/dev/null
    else
        warning "No blocks found in database. This might be due to PBFT consensus requiring multiple nodes."
    fi
else
    warning "Database file not created."
fi

exit $SUCCESS_EXITCODE
