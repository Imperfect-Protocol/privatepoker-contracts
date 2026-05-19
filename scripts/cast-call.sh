#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_vars.sh"

load_privatepoker_env
maybe_load_privatepoker_core_env
contract_call "$@"
