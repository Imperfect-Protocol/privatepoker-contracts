#!/bin/bash
set -euo pipefail

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
CONTRACTS_ROOT="$(cd "$SCRIPTS_DIR/.." && pwd)"
IMPERFECT_ROOT="$(cd "$CONTRACTS_ROOT/.." && pwd)"
CRUMBLE_CONFIG="$IMPERFECT_ROOT/crumble/crum_p2p_bot.toml"
ENV_FILE="$CONTRACTS_ROOT/.privatepoker.local.env"

cd "$CONTRACTS_ROOT"
. "$SCRIPTS_DIR/set_vars.sh"

require_env() {
    local name="$1"
    if [ -z "${!name:-}" ]; then
        die "Environment variable undefined: $name"
    fi
}

capture_address() {
    perl -pe 's/\e\[[0-9;]*[mK]//g' \
        | sed -nE 's/.*deployed code at address:[[:space:]]*(0x[0-9a-fA-F]{40}).*/\1/p' \
        | tail -n 1
}

ensure_wasm() {
    local contract="$1"
    "$SCRIPTS_DIR/check-contract.sh" "$contract" >&2
}

deploy_plain() {
    local contract="$1"
    echo "==> Deploying $contract" >&2
    if ! ensure_wasm "$contract"; then
        die "Failed to build contract: '$contract'"
    fi
    local output
    if ! output=$("$SCRIPTS_DIR/deploy-contract.sh" "$contract" 2>&1); then
        echo "$output" >&2
        die "Deploy failed for contract: '$contract'"
    fi
    echo "$output" >&2
    local address
    address=$(printf '%s\n' "$output" | capture_address)
    if [ -z "$address" ]; then
        die "Could not find deployed address for $contract"
    fi
    printf '%s\n' "$address"
}

deploy_constructed() {
    local contract="$1"
    local signature="$2"
    shift 2

    echo "==> Deploying $contract" >&2
    if ! ensure_wasm "$contract"; then
        die "Failed to build contract: '$contract'"
    fi
    local output
    if ! output=$("$SCRIPTS_DIR/construct-contract.sh" "$contract" "$signature" "$@" 2>&1); then
        echo "$output" >&2
        die "Deploy failed for contract: '$contract'"
    fi
    echo "$output" >&2
    local address
    address=$(printf '%s\n' "$output" | capture_address)
    if [ -z "$address" ]; then
        die "Could not find deployed address for $contract"
    fi
    printf '%s\n' "$address"
}

send_tx() {
    "$SCRIPTS_DIR/cast-send.sh" "$@"
}

write_env_file() {
    cat > "$ENV_FILE" <<EOF
export PP_LOBBY=$PP_LOBBY
export PP_CHIPS=$PP_CHIPS
export PP_CASHIER=$PP_CASHIER
export PP_SIGNAL=$PP_SIGNAL
export PP_TEST_USDC=$PP_TEST_USDC
export PP_VERIFY_SHUFFLE=$PP_VERIFY_SHUFFLE
export PP_VERIFY_UNMASKING=$PP_VERIFY_UNMASKING
EOF
}

write_bot_config() {
    cat > "$CRUMBLE_CONFIG" <<EOF
[network]
rpc_url = "$RPC_URL"
topology = "mesh"

[table]
mode = "join"
lobby_id = 1
table_id = 1
table_name = "Local P2P Demo"
buy_in = 0
max_players = 2

[contracts]
lobby = "$PP_LOBBY"
signal = "$PP_SIGNAL"
verify_shuffle = "$PP_VERIFY_SHUFFLE"
verify_unmasking = "$PP_VERIFY_UNMASKING"
EOF
}

require_env PP_PRIVATE_KEY
require_env PP_OWNER

echo "Priming Private Poker devnode"
echo "RPC_URL=$RPC_URL"
echo "PP_OWNER=$PP_OWNER"

PP_TEST_USDC=$(deploy_constructed test_usdc 'constructor(address)' "$PP_OWNER")
export PP_TEST_USDC

PP_CHIPS=$(deploy_constructed chips 'constructor(address)' "$PP_OWNER")
export PP_CHIPS

PP_LOBBY=$(deploy_constructed lobby 'constructor(address)' "$PP_OWNER")
export PP_LOBBY

PP_SIGNAL=$(deploy_plain signal)
export PP_SIGNAL

PP_VERIFY_SHUFFLE=$(deploy_plain verify_shuffle)
export PP_VERIFY_SHUFFLE

PP_VERIFY_UNMASKING=$(deploy_plain verify_unmasking)
export PP_VERIFY_UNMASKING

PP_CASHIER=$(deploy_constructed cashier 'constructor(address,address,address)' "$PP_OWNER" "$PP_TEST_USDC" "$PP_CHIPS")
export PP_CASHIER

echo "==> Wiring Chips/Cashier/Lobby"
send_tx "$PP_CHIPS" 'setCashier(address)' "$PP_CASHIER"
send_tx "$PP_CHIPS" 'setLobby(address)' "$PP_LOBBY"
send_tx "$PP_LOBBY" 'setChipToken(address)' "$PP_CHIPS"

echo "==> Creating lobby 1"
send_tx "$PP_LOBBY" 'addLobby(uint256,uint256,uint256,string)' 1 1 0 "Texas Hold'em"

write_env_file
write_bot_config

cat <<EOF

Private Poker devnode primed.

PP_LOBBY=$PP_LOBBY
PP_CHIPS=$PP_CHIPS
PP_CASHIER=$PP_CASHIER
PP_SIGNAL=$PP_SIGNAL
PP_TEST_USDC=$PP_TEST_USDC
PP_VERIFY_SHUFFLE=$PP_VERIFY_SHUFFLE
PP_VERIFY_UNMASKING=$PP_VERIFY_UNMASKING

Wrote:
  $ENV_FILE
  $CRUMBLE_CONFIG

To load addresses into your current shell:
  source $ENV_FILE
EOF
