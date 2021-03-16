#!/bin/bash
set -euo pipefail

cd $(realpath $(dirname $0))
source ./env.sh

VAULT_PGUSER=${VAULT_PGUSER:=vaultuser}
VAULT_PGPASSWORD=${VAULT_PGPASSWORD:=vaultuser}

vault auth enable approle

vault secrets enable database
vault write database/config/ergo-postgresql \
  plugin_name=postgresql-database-plugin \
  allowed_roles="*" \
  connection_url="postgresql://{{username}}:{{password}}@${DATABASE_HOST}:${DATABASE_PORT}/${DATABASE_NAME}" \
  username="${VAULT_PGUSER}" \
  password="${VAULT_PGPASSWORD}"


