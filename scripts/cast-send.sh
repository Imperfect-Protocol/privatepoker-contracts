#!/bin/bash

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. $SCRIPTS_DIR/set_vars.sh

cast_send() {
    if [ -z "$PP_PRIVATE_KEY" ]; then
        die "Environment variable undefined: PP_PRIVATE_KEY"
    fi

    CAST_ARGS=(--rpc-url $RPC_URL $1 "$2" "${@:3}")
    
    cast send --private-key $PP_PRIVATE_KEY "${CAST_ARGS[@]}"
}

cast_send $@