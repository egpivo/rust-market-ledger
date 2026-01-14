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

if ! command -v sqlite3 &> /dev/null; then
    error "sqlite3 not found. Please install SQLite3 first."
fi

NODE_ID=${1:-0}
DB_FILE="blockchain_node_${NODE_ID}.db"

if [ ! -f "$DB_FILE" ]; then
    error "Database file not found: $DB_FILE"
fi

info "Querying Node $NODE_ID database: $DB_FILE"
echo ""

TOTAL=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM blockchain;" 2>/dev/null)
info "Total blocks: $TOTAL"
echo ""

info "Latest blocks:"
sqlite3 -header -column "$DB_FILE" "SELECT block_index, substr(hash, 1, 16) as hash_preview, substr(data_json, 1, 60) as data_preview FROM blockchain ORDER BY block_index DESC LIMIT 10;" 2>/dev/null

echo ""
info "Block details (latest 3):"
sqlite3 -header -column "$DB_FILE" "SELECT block_index, timestamp, substr(prev_hash, 1, 16) as prev_hash_preview, substr(hash, 1, 16) as hash_preview, nonce FROM blockchain ORDER BY block_index DESC LIMIT 3;" 2>/dev/null

exit $SUCCESS_EXITCODE
