#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
. "$SCRIPTS_DIR/privatepoker_vars.sh"

check_all_contracts "$@"
