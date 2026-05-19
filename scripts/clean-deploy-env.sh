#!/bin/sh

. ./scripts/privatepoker_vars.sh || return 1 2>/dev/null || exit 1

clean_privatepoker_deploy_env "$@"
