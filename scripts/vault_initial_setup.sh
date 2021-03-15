#!/bin/bash
set -euo pipefail

cd $(realpath $(dirname $0))
source ./env.sh

vault secrets enable database
vault write database/config/ergo-postgresql \
  plugin_name=postgresql-database-plugin \
  allowed_roles="*" \
  connection_url="postgresql://{{username}}:{{password}}:${DATABASE_HOST}:${DATABASE_PORT}/" \
  username="vaultuser" \
  password="vaultpassword"


