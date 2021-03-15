#!/bin/bash

# Run Vault in development mode
set -e

cd $(realpath (dirname $0))

vault server -dev -dev-root-token-id=root &
./vault_roles.sh
fg

