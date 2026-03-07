#!/bin/bash

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. $SCRIPTS_DIR/set_vars.sh

set_vars $1

check_contract() {
    if [ "$#" -le 0 ]; then
        die "Missing arguments: CONTRACT_NAME [OPTIONAL ARGS...]"
    fi

    rm -f "$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH"

    # Build sources
    cd $CONTRACT_PATH && cargo stylus check || true 

    if [ ! -f "$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH" ]; then
        die "Failed to build contract: '$CONTRACT_NAME'"
    fi

    # Check using RPC URL
    cd $CONTRACT_PATH && cargo stylus check \
        --endpoint="$RPC_URL" \
        --wasm-file="$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH" \
        --source-files-for-project-hash="$CONTRACT_PATH"
}

check_contract $@
