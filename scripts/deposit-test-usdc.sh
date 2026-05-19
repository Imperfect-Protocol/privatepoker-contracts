#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_testvars.sh"

deposit_test_usdc "$@"
