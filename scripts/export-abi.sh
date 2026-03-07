#!/bin/bash

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. $SCRIPTS_DIR/set_vars.sh

set_vars $1

export_abi() {
    if [ "$#" -le 0 ]; then
        die "Missing arguments: CONTRACT_NAME [OPTIONAL ARGS...]"
    fi

    cd $CONTRACT_PATH && RUST_BACKTRACE=1 cargo stylus export-abi ${@:2}
}

export_abi $@