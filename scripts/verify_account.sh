#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_vars.sh"

if [ "$#" -ne 1 ]; then
    cat >&2 <<EOF
Usage: ./scripts/verify_account.sh PLAYER_ADDRESS

Sets the account status to Verified.
EOF
    exit 1
fi

load_privatepoker_env
maybe_load_privatepoker_core_env
require_env PP_LOBBY

contract_send "$PP_LOBBY" 'setAccountStatus(address,uint256)' "$1" 2
