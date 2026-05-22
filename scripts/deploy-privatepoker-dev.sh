#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_testvars.sh"

PRIVATEPOKER_DEV_USDC_CENTS=${PRIVATEPOKER_DEV_USDC_CENTS:-1000000}
PRIVATEPOKER_DEV_ETH_VALUE=${PRIVATEPOKER_DEV_ETH_VALUE:-1ether}

fund_dev_account_usdc() {
    account="$1"
    raw_amount=$((PRIVATEPOKER_DEV_USDC_CENTS * 10000))
    log_step "Minting ${PRIVATEPOKER_DEV_USDC_CENTS} cents (${raw_amount} raw TEST_USDC units) to ${account}"
    contract_send "$PP_TEST_USDC" 'mint(address,uint256)' "$account" "$raw_amount"
}

fund_dev_account_eth() {
    account="$1"
    log_step "Funding ${account} with ${PRIVATEPOKER_DEV_ETH_VALUE}"
    cast send \
        --private-key "$PP_PRIVATE_KEY" \
        --rpc-url "$RPC_URL" \
        --value "$PRIVATEPOKER_DEV_ETH_VALUE" \
        "$account"
}

load_privatepoker_env
require_env PP_PRIVATE_KEY
require_env PP_OWNER

if [ "$#" -eq 0 ]; then
    cat >&2 <<EOF
Usage: ./scripts/deploy-privatepoker-dev.sh ACCOUNT [ACCOUNT...]

Runs full local deployment, creates lobby 1, then funds exactly the accounts
passed as arguments with TEST_USDC and native gas.

Optional env:
  PRIVATEPOKER_DEV_USDC_CENTS=${PRIVATEPOKER_DEV_USDC_CENTS}
  PRIVATEPOKER_DEV_ETH_VALUE=${PRIVATEPOKER_DEV_ETH_VALUE}
EOF
    exit 1
fi

id=$(chain_id)
core_env=$(deployment_env_file "$id")
test_usdc_env=$(test_usdc_env_file "$id")

log_step "Cleaning previous dev deployment env files"
rm -f "$core_env" "$test_usdc_env"
unset PP_TEST_USDC PP_USDC PP_LOBBY PP_ACCOUNT PP_CASHIER PP_CHIPS
unset PP_LOBBY_FACET PP_TABLE_FACET PP_HAND_FACET PP_SPECTATE_FACET
unset PP_ACCOUNT_FACET PP_CASHIER_FACET PP_CHIPS_FACET
unset PP_SIGNAL PP_VERIFY_SHUFFLE PP_VERIFY_UNMASKING

log_step "Checking every Private Poker contract"
check_all_contracts

log_step "Deploying TEST_USDC"
deploy_privatepoker_test_usdc

log_step "Deploying Private Poker Diamond"
deploy_privatepoker_core

log_step "Creating default Texas Holdem lobby"
contract_send "$PP_LOBBY" 'addLobby(uint256,uint256,uint256,string)' 1 1 1 "Texas Holdem"

log_step "Funding requested accounts"
for account in "$@"; do
    fund_dev_account_usdc "$account"
    fund_dev_account_eth "$account"
done

echo
echo "Private Poker dev deployment complete."
echo
echo "PP_USDC=$PP_USDC"
echo "PP_LOBBY=$PP_LOBBY"
echo "PP_SIGNAL=$PP_SIGNAL"
echo
echo "Wrote env files:"
echo "  $test_usdc_env"
echo "  $core_env"
