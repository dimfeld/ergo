#!/bin/bash

# Run Vault in development mode
set -euo pipefail

cd $(realpath $(dirname $0))
source ./env.sh

vault server -dev -dev-root-token-id=root &
PID=$!

# Wait for Vault to actually start before we try to configure it.
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
# Set up the roles
# For dev purposes, run with looser restrictions on the secret ID since we'll need it
# again every time the server restarts.
SECRET_ID_PROPERTIES="secret_id_ttl=720h" ./vault_roles.sh || true

# For dev mode, just stick the role ID and secret IDs into a .env file that the servers can read.
# Definitely don't do this for production deploys.
rm -f ../vault_dev_roles.env
for role in ${VAULT_SINGLE_ROLES} ${VAULT_AIO_ROLE}; do
  ROLE_ID=$(VAULT_FORMAT=json vault read auth/approle/role/${role}/role-id | jq -r .data.role_id)
  SECRET_ID=$(VAULT_FORMAT=json vault write -f auth/approle/role/${role}/secret-id | jq -r .data.secret_id)

  UPPER_ROLE=$(echo "${role}" | tr '[:lower:]' '[:upper:]')
  cat >> ../vault_dev_roles.env <<EOF
  VAULT_ROLE_${UPPER_ROLE}_ID=${ROLE_ID}
  VAULT_ROLE_${UPPER_ROLE}_SECRET=${SECRET_ID}
EOF
done

# Put Vault in the foregroupnd so that ctrl+c will kill it.
wait $PID

