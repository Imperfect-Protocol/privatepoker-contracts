#!/bin/sh

PRIVATEPOKER_TEST_SCRIPTS_DIR="${PRIVATEPOKER_TEST_SCRIPTS_DIR:-${SCRIPTS_DIR:-./scripts}}"
. "$PRIVATEPOKER_TEST_SCRIPTS_DIR/privatepoker_vars.sh" || return 1 2>/dev/null || exit 1

test_usdc_env_file() {
    id="$1"
    if [ -n "${PP_TEST_USDC_ENV_FILE:-}" ]; then
        printf '%s\n' "$PP_TEST_USDC_ENV_FILE"
        return
    fi

    require_privatepoker_env_namespace || return 1
    printf '%s\n' "$PP_HOME/.privatepoker.$PP_ENV.test-usdc.$id.env"
}

load_privatepoker_test_usdc_env() {
    id=$(chain_id) || return 1
    env_file=$(test_usdc_env_file "$id") || return 1
    source_env_if_present "$env_file" || true
}

write_privatepoker_test_usdc_env() {
    env_file="$1"
    cat > "$env_file" <<EOF
export RPC_URL=$RPC_URL
export PP_OWNER=$PP_OWNER
export PP_TEST_USDC=$PP_TEST_USDC
export PP_USDC=$PP_TEST_USDC
EOF
}

deploy_privatepoker_test_usdc() {
    load_privatepoker_env || return 1
    require_env PP_PRIVATE_KEY || return 1
    require_env PP_OWNER || return 1

    id=$(chain_id) || return 1
    env_file=$(test_usdc_env_file "$id") || return 1

    if [ -f "$env_file" ] && [ "${PP_FORCE_TEST_USDC_DEPLOY:-0}" != "1" ]; then
        die "$env_file already exists. Set PP_FORCE_TEST_USDC_DEPLOY=1 to overwrite it, or set PP_TEST_USDC_ENV_FILE to another path."
        return 1
    fi

    log_step "Deploying Private Poker TEST_USDC"
    echo "RPC_URL=$RPC_URL" >&2
    echo "CHAIN_ID=$id" >&2
    echo "PP_OWNER=$PP_OWNER" >&2

    PP_TEST_USDC=$(capture_deployment deploy_constructed test_usdc 'constructor(address)' "$PP_OWNER") || return 1
    export PP_TEST_USDC

    write_privatepoker_test_usdc_env "$env_file" || return 1
    export_env_file "$env_file" || return 1

    echo
    echo "Private Poker TEST_USDC deployed."
    echo
    print_privatepoker_deploy_table_header
    print_privatepoker_deploy_row test_usdc PP_TEST_USDC "$PP_TEST_USDC"
    print_privatepoker_deploy_row test_usdc PP_USDC "$PP_TEST_USDC"
    echo
    printf 'Wrote:\n  %s\n' "$env_file"
}

fund_test_usdc() {
    load_privatepoker_env 1 || return 1
    load_privatepoker_test_usdc_env || return 1

    if [ "$#" -ne 2 ]; then
        cat >&2 <<EOF
Usage: ./scripts/fund-test-usdc.sh AMOUNT_CENTS RECIPIENT_ADDRESS

Mints TEST_USDC to RECIPIENT_ADDRESS using the configured PP_PRIVATE_KEY owner.
AMOUNT_CENTS is a dollar-cent amount. TEST_USDC has 6 decimals, so:
  ./scripts/fund-test-usdc.sh 100000 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266
mints 1000.00 TEST_USDC.
EOF
        return 1
    fi

    amount_cents="$1"
    recipient="$2"

    if ! printf '%s\n' "$amount_cents" | grep -Eq '^[0-9]+$'; then
        die "AMOUNT_CENTS must be a non-negative integer"
        return 1
    fi

    if ! printf '%s\n' "$recipient" | grep -Eq '^0x[0-9a-fA-F]{40}$'; then
        die "RECIPIENT_ADDRESS must be a 20-byte hex address"
        return 1
    fi

    require_env PP_PRIVATE_KEY || return 1
    require_env PP_TEST_USDC || return 1

    raw_amount=$((amount_cents * 10000))
    log_step "Minting ${amount_cents} cents (${raw_amount} raw TEST_USDC units) to ${recipient}"
    contract_send "$PP_TEST_USDC" 'mint(address,uint256)' "$recipient" "$raw_amount" || return 1
}

fund_test_usdc_to_test_accounts() {
    if [ "$#" -ne 1 ]; then
        cat >&2 <<EOF
Usage: ./scripts/fund-test-usdc-to-test-accounts.sh AMOUNT_CENTS

Mints AMOUNT_CENTS of TEST_USDC to each standard Anvil account.
Example:
  ./scripts/fund-test-usdc-to-test-accounts.sh 100000
mints 1000.00 TEST_USDC to each of the 10 standard Anvil accounts.
EOF
        return 1
    fi

    amount_cents="$1"
    for account in \
        0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 \
        0x70997970C51812dc3A010C7d01b50e0d17dc79C8 \
        0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC \
        0x90F79bf6EB2c4f870365E785982E1f101E93b906 \
        0x15d34AAf54267DB7D7c367839AAf71A00a2C6A65 \
        0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc \
        0x976EA74026E726554dB657fA54763abd0C3a0aa9 \
        0x14dC79964da2C08b23698B3D3cc7Ca32193d9955 \
        0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f \
        0xa0Ee7A142d267C1f36714E4a8F75612F20a79720
    do
        fund_test_usdc "$amount_cents" "$account" || return 1
    done
}
