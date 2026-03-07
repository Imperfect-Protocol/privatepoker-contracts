#!/bin/bash
set -e

die() {
    echo "Script error: $1" >&2
    exit 1
}

RPC_URL=${RPC_URL:-"http://localhost:8547"}
MAX_GAS_FEE=${MAX_GAS_FEE:-30}

if ! command -v jq &> /dev/null; then
    die "Please install 'jq'."
fi

CARGO_METADATA_COMMAND="cargo metadata --format-version 1 --no-deps"
CARGO_WORKSPACE_ROOT=$($CARGO_METADATA_COMMAND | jq -r '.workspace_root')

if [ -z "$CARGO_WORKSPACE_ROOT" ]; then
    die "Script must be ran from root cargo workspace directory."
fi

set_vars() {
    CONTRACT_NAME=${1:-$(basename "$PWD")}
    CONTRACT_PATH="$CARGO_WORKSPACE_ROOT/contracts/$CONTRACT_NAME"
    WASM_FILE_PATH="target/wasm32-unknown-unknown/release/$CONTRACT_NAME.wasm"

    if [ ! -d $CONTRACT_PATH ]; then
        die "Contract '$CONTRACT_NAME' not found ($CONTRACT_PATH)"
    fi

    echo "RPC_URL = $RPC_URL"
    echo "MAX_GAS_FEE= $MAX_GAS_FEE"
    echo "CARGO_WORKSPACE_ROOT = $CARGO_WORKSPACE_ROOT"
    echo "CONTRACT_NAME = $CONTRACT_NAME"
    echo "CONTRACT_PATH = $CONTRACT_PATH"
    echo "WASM_FILE_PATH = $WASM_FILE_PATH"
    echo "---"
}





