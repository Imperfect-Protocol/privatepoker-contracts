#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_vars.sh"

if [ "$#" -lt 2 ] || [ "$#" -gt 3 ]; then
    cat >&2 <<EOF
Usage: ./scripts/create_account.sh PLAYER_ADDRESS DISPLAY_NAME [ENCRYPTED_PROFILE_HEX]

Creates an Unverified account for PLAYER_ADDRESS.
EOF
    exit 1
fi

load_privatepoker_env
maybe_load_privatepoker_core_env
require_env PP_LOBBY

PROFILE_HEX="${3:-0x}"

contract_send "$PP_LOBBY" 'createAccount(address,string,bytes)' "$1" "$2" "$PROFILE_HEX"
