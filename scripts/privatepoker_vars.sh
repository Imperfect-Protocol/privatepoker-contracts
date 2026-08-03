#!/bin/sh

die() {
    echo "Script error: $1" >&2
    return 1
}

PRIVATEPOKER_SCRIPTS_DIR="${PRIVATEPOKER_SCRIPTS_DIR:-${SCRIPTS_DIR:-./scripts}}"
PRIVATEPOKER_CONTRACTS_ROOT="$(cd "$PRIVATEPOKER_SCRIPTS_DIR/.." && pwd)"

log_step() {
    echo "==> $*" >&2
}

require_env() {
    name="$1"
    eval "value=\${$name:-}"
    if [ -z "$value" ]; then
        die "Environment variable undefined: $name"
        return 1
    fi
}

load_privatepoker_env() {
    fallback_local="${1:-0}"
    if [ -n "${PP_ENV_FILE:-}" ]; then
        log_step "Loading $PP_ENV_FILE"
        . "$PP_ENV_FILE"
    elif [ "$fallback_local" = "1" ] && [ -f "$PRIVATEPOKER_CONTRACTS_ROOT/.privatepoker.local.env" ]; then
        log_step "Loading $PRIVATEPOKER_CONTRACTS_ROOT/.privatepoker.local.env"
        . "$PRIVATEPOKER_CONTRACTS_ROOT/.privatepoker.local.env"
    fi
}

source_env_if_present() {
    env_file="$1"
    if [ -f "$env_file" ]; then
        log_step "Loading $env_file"
        . "$env_file"
        return 0
    fi
    return 1
}

require_privatepoker_env_namespace() {
    require_env PP_HOME || return 1
    require_env PP_ENV || return 1
    mkdir -p "$PP_HOME" || return 1
}

parse_deployment_address() {
    awk -F: '/deployed code at address:/ {
        gsub(/\x1b\[[0-9;]*m/, "", $2);
        sub(/^[ \t]+/, "", $2);
        sub(/[ \t]+$/, "", $2);
        print $2
    }'
}

RPC_URL=${RPC_URL:-"http://localhost:8547"}
MAX_GAS_FEE=${MAX_GAS_FEE:-30}

if ! command -v jq >/dev/null 2>&1; then
    die "Please install 'jq'."
    return 1 2>/dev/null || exit 1
fi

CARGO_WORKSPACE_ROOT=$(
    cargo metadata \
        --manifest-path "$PRIVATEPOKER_CONTRACTS_ROOT/Cargo.toml" \
        --format-version 1 \
        --no-deps \
        | jq -r '.workspace_root'
)

if [ -z "$CARGO_WORKSPACE_ROOT" ]; then
    die "Script must be ran from root cargo workspace directory."
    return 1 2>/dev/null || exit 1
fi

set_vars() {
    CONTRACT_NAME=${1:-$(basename "$PWD")}
    CONTRACT_PATH="$CARGO_WORKSPACE_ROOT/contracts/$CONTRACT_NAME"
    WASM_FILE_PATH="target/wasm32-unknown-unknown/release/$CONTRACT_NAME.wasm"

    if [ ! -d "$CONTRACT_PATH" ]; then
        die "Contract '$CONTRACT_NAME' not found ($CONTRACT_PATH)"
        return 1
    fi

    echo "RPC_URL = $RPC_URL" >&2
    echo "MAX_GAS_FEE= $MAX_GAS_FEE" >&2
    echo "CARGO_WORKSPACE_ROOT = $CARGO_WORKSPACE_ROOT" >&2
    echo "CONTRACT_NAME = $CONTRACT_NAME" >&2
    echo "CONTRACT_PATH = $CONTRACT_PATH" >&2
    echo "WASM_FILE_PATH = $WASM_FILE_PATH" >&2
    echo "---" >&2
}

check() {
    if [ "$#" -le 0 ]; then
        die "Missing arguments: CONTRACT_NAME [OPTIONAL ARGS...]"
        return 1
    fi

    set_vars "$1" || return 1
    rm -f "$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH"

    cd "$CONTRACT_PATH" && cargo stylus check >/dev/null 2>&1 || true

    if [ ! -f "$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH" ]; then
        die "Failed to build contract: '$CONTRACT_NAME'"
        return 1
    fi

    check_output="$CARGO_WORKSPACE_ROOT/target/privatepoker-stylus-check.$CONTRACT_NAME.$$.log"
    if command -v script >/dev/null 2>&1; then
        if cd "$CONTRACT_PATH" && CARGO_TERM_COLOR=always script -q "$check_output" cargo stylus check \
            --endpoint="$RPC_URL" \
            --wasm-file="$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH" \
            --source-files-for-project-hash="$CONTRACT_PATH"
        then
            check_status=0
        else
            check_status=$?
        fi
    else
        if cd "$CONTRACT_PATH" && CARGO_TERM_COLOR=always cargo stylus check \
            --endpoint="$RPC_URL" \
            --wasm-file="$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH" \
            --source-files-for-project-hash="$CONTRACT_PATH" > "$check_output" 2>&1
        then
            check_status=0
        else
            check_status=$?
        fi
        cat "$check_output" >&2
    fi

    ansi_escape=$(printf '\033')
    size_values=$(
        awk -v esc="$ansi_escape" '{
            raw = $0;
            line = raw;
            sgr = esc "\\[[0-9;]*m";
            color = "";
            remaining = raw;
            while (match(remaining, sgr)) {
                code = substr(remaining, RSTART, RLENGTH);
                if (code != esc "[0m") {
                    color = color code;
                }
                remaining = substr(remaining, RSTART + RLENGTH);
            }
            gsub(sgr, "", line);
            if (line ~ /contract size:/) {
                split(line, fields, " ");
                kib = fields[3];
                bytes = fields[5];
                gsub(/\(/, "", bytes);
                gsub(/\)/, "", bytes);
                found = 1;
            }
        }
        END {
            if (found) {
                printf "%s %s %s\n", bytes, kib, color;
            }
        }' "$check_output"
    )

    rm -f "$check_output"

    if [ -n "$size_values" ] && [ -n "${PRIVATEPOKER_CHECK_SIZES_FILE:-}" ]; then
        set -- $size_values
        printf '%s\t%s\t%s\t%s\n' "$CONTRACT_NAME" "$1" "$2" "$3" >> "$PRIVATEPOKER_CHECK_SIZES_FILE"
    fi
    if [ -n "$size_values" ]; then
        size_cache="$CARGO_WORKSPACE_ROOT/target/privatepoker-contract-sizes.latest.tsv"
        size_cache_tmp="$size_cache.$$"
        if [ -f "$size_cache" ]; then
            awk -F '	' -v name="$CONTRACT_NAME" '$1 != name { print }' "$size_cache" > "$size_cache_tmp"
        else
            : > "$size_cache_tmp"
        fi
        set -- $size_values
        printf '%s\t%s\t%s\t%s\n' "$CONTRACT_NAME" "$1" "$2" "$3" >> "$size_cache_tmp"
        mv "$size_cache_tmp" "$size_cache"
    fi

    return "$check_status"
}

all_contract_names() {
    printf '%s\n' \
        account \
        cashier \
        chips \
        diamond \
        hand \
        lobby \
        spectate \
        table \
        test_usdc \
        verify_shuffle \
        verify_unmasking
}

check_all_contracts() {
    PRIVATEPOKER_CHECK_SIZES_FILE="$CARGO_WORKSPACE_ROOT/target/privatepoker-contract-sizes.$$.tsv"
    export PRIVATEPOKER_CHECK_SIZES_FILE
    rm -f "$PRIVATEPOKER_CHECK_SIZES_FILE"

    for contract in $(all_contract_names); do
        log_step "Checking $contract"
        check "$contract"
    done

    ansi_bold="$(printf '\033[1m')"
    ansi_green="$(printf '\033[32m')"
    ansi_yellow="$(printf '\033[33m')"
    ansi_red="$(printf '\033[31m')"
    ansi_reset="$(printf '\033[0m')"
    size_yellow_bytes=${PRIVATEPOKER_CONTRACT_SIZE_YELLOW_BYTES:-18432}
    size_red_bytes=${PRIVATEPOKER_CONTRACT_SIZE_RED_BYTES:-24576}

    echo
    printf '%s%-22s %8s %8s%s\n' "$ansi_bold" "name" "bytes" "KiB" "$ansi_reset"
    printf '%-22s %8s %8s\n' "----" "-----" "---"
    if [ -f "$PRIVATEPOKER_CHECK_SIZES_FILE" ]; then
        while IFS='	' read -r name bytes kib row_color; do
            if [ "$bytes" -ge "$size_red_bytes" ]; then
                row_color="$ansi_red"
            elif [ "$bytes" -ge "$size_yellow_bytes" ]; then
                row_color="$ansi_yellow"
            else
                row_color="$ansi_green"
            fi
            printf '%-22s %8s %s%8s%s\n' "$name" "$bytes" "$row_color" "$kib" "$ansi_reset"
        done < "$PRIVATEPOKER_CHECK_SIZES_FILE"
    fi
    echo
    rm -f "$PRIVATEPOKER_CHECK_SIZES_FILE"
    unset PRIVATEPOKER_CHECK_SIZES_FILE
}

deploy() {
    if [ "$#" -le 0 ]; then
        die "Missing arguments: CONTRACT_NAME [DEPLOY_ARGS...]"
        return 1
    fi

    contract="$1"
    shift
    set_vars "$contract" || return 1
    require_env PP_PRIVATE_KEY || return 1

    check "$CONTRACT_NAME" || return 1

    echo "cargo stylus deploy --private-key \$PP_PRIVATE_KEY --endpoint=$RPC_URL --wasm-file=$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH --max-fee-per-gas-gwei=$MAX_GAS_FEE --no-verify $*" >&2
    cd "$CARGO_WORKSPACE_ROOT" && cargo stylus deploy \
        --private-key="$PP_PRIVATE_KEY" \
        --endpoint="$RPC_URL" \
        --wasm-file="$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH" \
        --max-fee-per-gas-gwei="$MAX_GAS_FEE" \
        --no-verify \
        "$@"
}

deploy_constructed() {
    if [ "$#" -le 1 ]; then
        die "Missing arguments: CONTRACT_NAME CONSTRUCTOR_SIGNATURE [CONSTRUCTOR_ARGS...]"
        return 1
    fi

    contract="$1"
    signature="$2"
    shift 2
    set_vars "$contract" || return 1
    require_env PP_PRIVATE_KEY || return 1

    check "$CONTRACT_NAME" || return 1

    constructor_args=""
    for arg in "$@"; do
        constructor_args="$constructor_args --constructor-args=$arg"
    done

    echo "cargo stylus deploy --private-key \$PP_PRIVATE_KEY --endpoint=$RPC_URL --wasm-file=$CARGO_WORKSPACE_ROOT/$WASM_FILE_PATH --max-fee-per-gas-gwei=$MAX_GAS_FEE --no-verify --constructor-signature=$signature$constructor_args" >&2
    cd "$CARGO_WORKSPACE_ROOT" && eval "cargo stylus deploy \
        --private-key=\"\$PP_PRIVATE_KEY\" \
        --endpoint=\"\$RPC_URL\" \
        --wasm-file=\"\$CARGO_WORKSPACE_ROOT/\$WASM_FILE_PATH\" \
        --max-fee-per-gas-gwei=\"\$MAX_GAS_FEE\" \
        --no-verify \
        --constructor-signature=\"\$signature\" \
        $constructor_args"
}

export_abi() {
    if [ "$#" -le 0 ]; then
        die "Missing arguments: CONTRACT_NAME [OPTIONAL ARGS...]"
        return 1
    fi

    contract="$1"
    shift
    set_vars "$contract" || return 1
    cd "$CONTRACT_PATH" && RUST_BACKTRACE=1 cargo stylus export-abi "$@"
}

contract_send() {
    if [ "$#" -le 1 ]; then
        die "Missing arguments: CONTRACT_ADDRESS FUNCTION_SIGNATURE [ARGS...]"
        return 1
    fi

    require_env PP_PRIVATE_KEY || return 1
    echo "cast send --private-key \$PP_PRIVATE_KEY --rpc-url $RPC_URL $*" >&2
    cast send --private-key "$PP_PRIVATE_KEY" --rpc-url "$RPC_URL" "$@"
}

contract_call() {
    if [ "$#" -le 1 ]; then
        die "Missing arguments: CONTRACT_ADDRESS FUNCTION_SIGNATURE [ARGS...]"
        return 1
    fi

    echo "cast call --rpc-url $RPC_URL $*" >&2
    cast call --rpc-url "$RPC_URL" "$@"
}

chain_id() {
    cast chain-id --rpc-url "$RPC_URL"
}

deployment_env_file() {
    id="$1"
    if [ -n "${PP_DEPLOYMENT_ENV_FILE:-}" ]; then
        printf '%s\n' "$PP_DEPLOYMENT_ENV_FILE"
        return
    fi

    require_privatepoker_env_namespace || return 1
    printf '%s\n' "$PP_HOME/.privatepoker.$PP_ENV.$id.env"
}

load_privatepoker_core_env() {
    id=$(chain_id)
    env_file=$(deployment_env_file "$id")
    source_env_if_present "$env_file" || true
}

load_privatepoker_test_usdc_env_file() {
    id="$1"
    if [ -n "${PP_TEST_USDC_ENV_FILE:-}" ]; then
        source_env_if_present "$PP_TEST_USDC_ENV_FILE" || true
        return
    fi

    if [ -n "${PP_HOME:-}" ] && [ -n "${PP_ENV:-}" ]; then
        source_env_if_present "$PP_HOME/.privatepoker.$PP_ENV.test-usdc.$id.env" || true
    fi
}

maybe_load_privatepoker_core_env() {
    if [ -n "${PP_HOME:-}" ] && [ -n "${PP_ENV:-}" ]; then
        load_privatepoker_core_env
    fi
}

clean_privatepoker_deploy_env() {
    load_privatepoker_env || return 1
    require_env PP_HOME || return 1
    require_env PP_ENV || return 1

    if [ "$#" -gt 1 ]; then
        cat >&2 <<EOF
Usage: ./scripts/clean-deploy-env.sh [CHAIN_ID]

Removes generated Private Poker deployment env files:
  \$PP_HOME/.privatepoker.\$PP_ENV.CHAIN_ID.env
  \$PP_HOME/.privatepoker.\$PP_ENV.test-usdc.CHAIN_ID.env

If CHAIN_ID is omitted, it is read from the configured RPC_URL.
EOF
        return 1
    fi

    if [ "$#" -eq 1 ]; then
        id="$1"
    else
        id=$(chain_id) || return 1
    fi

    core_env=$(deployment_env_file "$id") || return 1
    test_usdc_env="${PP_TEST_USDC_ENV_FILE:-$PP_HOME/.privatepoker.$PP_ENV.test-usdc.$id.env}"

    echo "Cleaning Private Poker deployment env for chain $id"
    for env_file in "$core_env" "$test_usdc_env"; do
        if [ -f "$env_file" ]; then
            rm -f "$env_file" || return 1
            echo "removed $env_file"
        else
            echo "missing $env_file"
        fi
    done
}

capture_deployment() {
    address=$("$@" | tee /dev/stderr | parse_deployment_address | tail -n 1)
    if ! printf '%s\n' "$address" | grep -Eq '^0x[0-9a-fA-F]{40}$'; then
        die "Could not parse deployment address from command: $*"
        return 1
    fi
    printf '%s\n' "$address"
}

export_env_file() {
    . "$1"
}

contract_size_from_cache() {
    contract="$1"
    size_cache="$CARGO_WORKSPACE_ROOT/target/privatepoker-contract-sizes.latest.tsv"
    if [ -f "$size_cache" ]; then
        awk -F '	' -v name="$contract" '$1 == name {
            bytes = $2;
            kib = $3;
            gsub(/\\t/, " ", bytes);
            split(bytes, parts, " ");
            if (parts[2] != "") {
                bytes = parts[1];
                kib = parts[2];
            }
            print bytes " " kib;
            found = 1;
            exit;
        }
        END {
            if (!found) {
                exit 1;
            }
        }' "$size_cache"
        return $?
    fi
    return 1
}

contract_size_from_wasm() {
    contract="$1"
    wasm_file="$CARGO_WORKSPACE_ROOT/target/wasm32-unknown-unknown/release/$contract.wasm"
    if [ -f "$wasm_file" ]; then
        bytes=$(wc -c < "$wasm_file" | tr -d ' ')
        kib=$(awk "BEGIN { printf \"%.1f\", $bytes / 1024 }")
        printf '%s %s\n' "$bytes" "$kib"
        return 0
    fi
    printf '%s %s\n' "-" "-"
}

print_privatepoker_deploy_table_header() {
    ansi_bold="$(printf '\033[1m')"
    ansi_reset="$(printf '\033[0m')"

    printf '%s%-22s %8s %8s  %-20s %s%s\n' "$ansi_bold" "name" "bytes" "KiB" "env" "address" "$ansi_reset"
    printf '%-22s %8s %8s  %-20s %s\n' "----" "-----" "---" "---" "-------"
}

print_privatepoker_deploy_row() {
    name="$1"
    env_name="$2"
    address="$3"

    size_values=$(contract_size_from_cache "$name" || contract_size_from_wasm "$name") || return 1
    bytes=$(printf '%s\n' "$size_values" | awk '{ print $1 }')
    kib=$(printf '%s\n' "$size_values" | awk '{ print $2 }')

    ansi_green="$(printf '\033[32m')"
    ansi_yellow="$(printf '\033[33m')"
    ansi_red="$(printf '\033[31m')"
    ansi_reset="$(printf '\033[0m')"
    size_yellow_bytes=${PRIVATEPOKER_CONTRACT_SIZE_YELLOW_BYTES:-18432}
    size_red_bytes=${PRIVATEPOKER_CONTRACT_SIZE_RED_BYTES:-24576}

    if [ "$bytes" = "-" ]; then
        kib_color=""
        ansi_reset=""
    elif [ "$bytes" -ge "$size_red_bytes" ]; then
        kib_color="$ansi_red"
    elif [ "$bytes" -ge "$size_yellow_bytes" ]; then
        kib_color="$ansi_yellow"
    else
        kib_color="$ansi_green"
    fi

    printf '%-22s %8s %s%8s%s  %-20s %s\n' "$name" "$bytes" "$kib_color" "$kib" "$ansi_reset" "$env_name" "$address"
}

print_privatepoker_core_deployment_summary() {
    echo
    echo "Private Poker core contracts deployed."
    echo
    print_privatepoker_deploy_table_header
    print_privatepoker_deploy_row diamond PP_LOBBY "$PP_LOBBY"
    print_privatepoker_deploy_row lobby PP_LOBBY_FACET "$PP_LOBBY_FACET"
    print_privatepoker_deploy_row table PP_TABLE_FACET "$PP_TABLE_FACET"
    print_privatepoker_deploy_row hand PP_HAND_FACET "$PP_HAND_FACET"
    print_privatepoker_deploy_row spectate PP_SPECTATE_FACET "$PP_SPECTATE_FACET"
    print_privatepoker_deploy_row account PP_ACCOUNT_FACET "$PP_ACCOUNT_FACET"
    print_privatepoker_deploy_row cashier PP_CASHIER_FACET "$PP_CASHIER_FACET"
    print_privatepoker_deploy_row chips PP_CHIPS_FACET "$PP_CHIPS_FACET"
    print_privatepoker_deploy_row verify_shuffle PP_VERIFY_SHUFFLE "$PP_VERIFY_SHUFFLE"
    print_privatepoker_deploy_row verify_unmasking PP_VERIFY_UNMASKING "$PP_VERIFY_UNMASKING"
    echo
}

write_privatepoker_core_env() {
    env_file="$1"
    cat > "$env_file" <<EOF
export RPC_URL=$RPC_URL
export PP_OWNER=$PP_OWNER
export PP_USDC=$PP_USDC
export PP_LOBBY=$PP_LOBBY
export PP_LOBBY_FACET=$PP_LOBBY_FACET
export PP_TABLE_FACET=$PP_TABLE_FACET
export PP_HAND_FACET=$PP_HAND_FACET
export PP_SPECTATE_FACET=$PP_SPECTATE_FACET
export PP_ACCOUNT=$PP_ACCOUNT
export PP_ACCOUNT_FACET=$PP_ACCOUNT_FACET
export PP_CHIPS=$PP_CHIPS
export PP_CHIPS_FACET=$PP_CHIPS_FACET
export PP_CASHIER=$PP_CASHIER
export PP_CASHIER_FACET=$PP_CASHIER_FACET
export PP_VERIFY_SHUFFLE=$PP_VERIFY_SHUFFLE
export PP_VERIFY_UNMASKING=$PP_VERIFY_UNMASKING
EOF
}

deploy_privatepoker_core() {
    load_privatepoker_env || return 1
    require_env PP_PRIVATE_KEY || return 1
    require_env PP_OWNER || return 1

    id=$(chain_id) || return 1
    load_privatepoker_test_usdc_env_file "$id" || return 1
    require_env PP_USDC || return 1

    env_file=$(deployment_env_file "$id") || return 1

    if [ -f "$env_file" ] && [ "${PP_FORCE_PRIVATEPOKER_DEPLOY:-0}" != "1" ]; then
        die "$env_file already exists. Set PP_FORCE_PRIVATEPOKER_DEPLOY=1 to overwrite it, or set PP_DEPLOYMENT_ENV_FILE to another path."
        return 1
    fi

    log_step "Deploying Private Poker core contracts"
    echo "RPC_URL=$RPC_URL" >&2
    echo "CHAIN_ID=$id" >&2
    echo "PP_OWNER=$PP_OWNER" >&2
    echo "PP_USDC=$PP_USDC" >&2

    PP_CHIPS_FACET=$(capture_deployment deploy_constructed chips 'constructor(address)' "$PP_OWNER") || return 1
    export PP_CHIPS_FACET

    PP_CASHIER_FACET=$(capture_deployment deploy_constructed cashier 'constructor(address,address,address)' "$PP_OWNER" "$PP_USDC" "$PP_CHIPS_FACET") || return 1
    export PP_CASHIER_FACET

    PP_ACCOUNT_FACET=$(capture_deployment deploy_constructed account 'constructor(address,address,address)' "$PP_OWNER" "$PP_USDC" "$PP_CHIPS_FACET") || return 1
    export PP_ACCOUNT_FACET

    PP_LOBBY_FACET=$(capture_deployment deploy_constructed lobby 'constructor(address)' "$PP_OWNER") || return 1
    export PP_LOBBY_FACET

    PP_TABLE_FACET=$(capture_deployment deploy table) || return 1
    export PP_TABLE_FACET

    PP_HAND_FACET=$(capture_deployment deploy hand) || return 1
    export PP_HAND_FACET

    PP_SPECTATE_FACET=$(capture_deployment deploy spectate) || return 1
    export PP_SPECTATE_FACET

    PP_VERIFY_SHUFFLE=$(capture_deployment deploy verify_shuffle) || return 1
    export PP_VERIFY_SHUFFLE

    PP_VERIFY_UNMASKING=$(capture_deployment deploy verify_unmasking) || return 1
    export PP_VERIFY_UNMASKING

    PP_LOBBY=$(capture_deployment deploy_constructed diamond 'constructor(address,address,address,address,address,address,address,address,address,address,address)' "$PP_OWNER" "$PP_LOBBY_FACET" "$PP_TABLE_FACET" "$PP_HAND_FACET" "$PP_SPECTATE_FACET" "$PP_ACCOUNT_FACET" "$PP_CASHIER_FACET" "$PP_CHIPS_FACET" "$PP_VERIFY_SHUFFLE" "$PP_VERIFY_UNMASKING" "$PP_USDC") || return 1
    export PP_LOBBY
    PP_ACCOUNT=$PP_LOBBY
    PP_CASHIER=$PP_LOBBY
    PP_CHIPS=$PP_LOBBY
    export PP_ACCOUNT
    export PP_CASHIER
    export PP_CHIPS

    write_privatepoker_core_env "$env_file" || return 1
    export_env_file "$env_file" || return 1

    print_privatepoker_core_deployment_summary
    printf 'Wrote:\n  %s\n' "$env_file"
}
