#!/bin/bash

# Run Vault in development mode
set -euo pipefail

cd $(realpath (dirname $0))

vault server -dev -dev-root-token-id=root &
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=root
./vault_initial_setup.sh
./vault_roles.sh
# Put it in the foregroupnd so that ctrl+c will kill it.
fg

