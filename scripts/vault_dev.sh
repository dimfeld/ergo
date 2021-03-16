#!/bin/bash

# Run Vault in development mode
set -euo pipefail

cd $(realpath $(dirname $0))
source ./env.sh

vault server -dev -dev-root-token-id=root &
PID=$!

RETRIES=0
MAX_RETRIES=10
WAIT=1
vault status &> /dev/null && WAIT=0
while [ $WAIT -ne 0 ] && [ $RETRIES -lt $MAX_RETRIES ]; do
  sleep 1
  vault status &> /dev/null && WAIT=0
  RETRIES=$((RETRIES + 1))
done

if [ $RETRIES -ge $MAX_RETRIES ]; then
  echo "Timed out waiting for Vault to start"
  exit 1
fi

./vault_initial_setup.sh || true
./vault_roles.sh || true

# Put it in the foregroupnd so that ctrl+c will kill it.
wait $PID

