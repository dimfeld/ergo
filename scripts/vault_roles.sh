#!/bin/bash
set -euo pipefail
cd $(realpath $(dirname $0))
source ./env.sh

# Configure Vault to work with Ergo
ROLES="ergo_web ergo_queues ergo_actions"

for role in $ROLES; do
  vault write database/roles/${role} \
      db_name=ergo-postgresql \
      creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' \
        VALID UNTIL '{{expiration}}'; \
        GRANT ${role} TO \"{{name}}\";" \
      revocation_statements="REVOKE ${role} FROM \"{{name}}\"; DROP ROLE IF EXISTS \"{{name}}\"" \
      default_ttl="1h" \
      max_ttl="8h"
done
