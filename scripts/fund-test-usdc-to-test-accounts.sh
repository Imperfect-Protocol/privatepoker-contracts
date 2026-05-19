#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_testvars.sh"

fund_test_usdc_to_test_accounts "$@"
