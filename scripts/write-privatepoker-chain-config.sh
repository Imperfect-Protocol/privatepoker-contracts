#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_vars.sh"

usage() {
    cat >&2 <<EOF
Usage: ./scripts/write-privatepoker-chain-config.sh OUTPUT_FILE

Writes a hardcoded Poker App chain config file.
EOF
}

if [ "$#" -ne 1 ]; then
    usage
    exit 1
fi

output_file="$1"

load_privatepoker_env 1 || true

detected_chain_id=""
if detected_chain_id=$(chain_id 2>/dev/null); then
    :
else
    detected_chain_id=""
fi

case "$detected_chain_id" in
    412346)
        chain_key="devnode"
        ;;
    421614)
        chain_key="sepolia"
        ;;
    *)
        case "${PP_ENV:-}" in
            devnode|nitro-devnode)
                chain_key="devnode"
                ;;
            sepolia|arbitrum-sepolia)
                chain_key="sepolia"
                ;;
            *)
                die "Could not determine chain. Set RPC_URL to devnode/sepolia or set PP_ENV to devnode/sepolia."
                exit 1
                ;;
        esac
        ;;
esac

if [ "$chain_key" = "devnode" ]; then
    chain_id="412_346"
    env_chain_id="412346"
    rpc_url="/local-rpc"
    polling_interval="3000"
    stale_time="5000"
    max_gas_block="{
    privatePoker: {
      default: 5_000_000n,
      functions: {
        setTableAggregatePublicKey: 10_000_000n,
        settleHand: 50_000_000n,
      },
    },
  }"
else
    chain_id="421_614"
    env_chain_id="421614"
    rpc_url="https://sepolia-rollup.arbitrum.io/rpc"
    polling_interval="6000"
    stale_time="10000"
    max_gas_block="{
    privatePoker: {
      default: 5_000_000n,
      functions: {
        setTableAggregatePublicKey: 10_000_000n,
        settleHand: 50_000_000n,
      },
    },
  }"
fi

if [ -n "${PP_HOME:-}" ] && [ -n "${PP_ENV:-}" ]; then
    source_env_if_present "$PP_HOME/.privatepoker.$PP_ENV.test-usdc.$env_chain_id.env" || true
    source_env_if_present "$PP_HOME/.privatepoker.$PP_ENV.$env_chain_id.env" || true
fi

mkdir -p "$(dirname "$output_file")"

cat > "$output_file" <<EOF
export const chainConfig = {
  key: '$chain_key',
  chainId: $chain_id,
  rpcUrl: '$rpc_url',
  ppdcContractAddress: '${PP_USDC:-}',
  privatePokerContractAddress: '${PP_LOBBY:-}',
  settlerFacetAddress: '${PP_SETTLER_FACET:-}',
  aggregatePubKeyFacetAddress: '${PP_AGGREGATE_PUB_KEY_FACET:-}',
  signatoryAddress: '${PP_SIGNATORY:-}',
  hashToCurveAddress: '${PP_HASH_TO_CURVE:-}',
  verifySignatureAddress: '${PP_VERIFY_SIGNATURE:-}',
  maxGas: $max_gas_block,
  pollingInterval: $polling_interval,
  staleTime: $stale_time,
  lastModified: '20260522T161800Z',
};
EOF

printf 'Wrote %s\n' "$output_file"
