#!/bin/bash

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. $SCRIPTS_DIR/set_vars.sh

set_vars $1

construct_contract() {
    if [ "$#" -le 1 ]; then
        die "Missing arguments: CONTRACT_NAME CONSTRUCTOR_SIGNATURE [CONSTRUCTOR_ARGS...]"
    fi

    set_vars $1

    if [ -z "$PP_PRIVATE_KEY" ]; then
        die "Environment variable undefined: PP_PRIVATE_KEY"
    fi

    if [ ! -f "./$WASM_FILE_PATH" ]; then
        check $CONTRACT_NAME
    fi

    STYLUS_ARGS=(
        --endpoint="$RPC_URL" \
        --wasm-file="./$WASM_FILE_PATH" \
        --max-fee-per-gas-gwei=$MAX_GAS_FEE \
        --no-verify \
        --constructor-signature="$2"
    )

    for arg in "${@:3}"; do
        STYLUS_ARGS+=(--constructor-args="$arg")
    done

    cd $CARGO_WORKSPACE_ROOT && cargo stylus deploy --private-key="$PP_PRIVATE_KEY" "${STYLUS_ARGS[@]}"
}

construct_contract "$@"
