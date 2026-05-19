#!/bin/sh

. ./scripts/privatepoker_testvars.sh || return 1 2>/dev/null || exit 1

deploy_privatepoker_test_usdc "$@"
