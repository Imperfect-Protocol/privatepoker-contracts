#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_vars.sh"

if [ "$#" -ne 1 ]; then
    cat >&2 <<EOF
Usage: ./scripts/create_account.sh PLAYER_ADDRESS

Creates an Unverified account for PLAYER_ADDRESS.
EOF
    exit 1
fi

load_privatepoker_env
maybe_load_privatepoker_core_env
require_env PP_LOBBY

contract_send "$PP_LOBBY" 'createAccount(address)' "$1"
