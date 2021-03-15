#!/bin/bash
set -euo pipefail
# Configure Vault to work with Ergo
ROLES=ergo_web ergo_queues ergo_actions

for role in ROLES; do
  vault write database/roles/${role} \
      db_name=ergo-postgresql \
      creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' \
        VALID UNTIL '{{expiration}}';
        GRANT ${role} TO \"{{name}}\";" \
      default_ttl="1h"
      max_ttl="24h"
done
