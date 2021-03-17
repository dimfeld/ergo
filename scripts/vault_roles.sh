#!/bin/bash
set -euo pipefail
cd $(realpath $(dirname $0))
source ./env.sh


# Be strict about secret ID usage in production environments
SECRET_ID_PROPERTIES=${SECRET_ID_PROPERTIES:=secret_id_ttl=10m secret_id_num_uses=1}

# Configure Vault to work with Ergo

for role in $VAULT_SINGLE_ROLES; do
  vault write database/roles/${role} \
      db_name=ergo-postgresql \
      creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' \
        VALID UNTIL '{{expiration}}'; \
        GRANT ${role} TO \"{{name}}\";" \
      revocation_statements="REVOKE ${role} FROM \"{{name}}\"; DROP ROLE IF EXISTS \"{{name}}\"" \
      default_ttl="${VAULT_DATABASE_ROLE_TTL}" \
      max_ttl="${VAULT_DATABASE_ROLE_MAX_TTL}"

  vault policy write ${role} - <<EOF
path "database/creds/${role}" {
  capabilities = [ "read" ]
}
EOF

  vault write auth/approle/role/${role} \
    token_policies="${role}" \
    token_period=15m \
    ${SECRET_ID_PROPERTIES}
done

# The all-in-one server gets a special approle that can access all the database credentials.
COMMAED_ROLES=$(echo "${VAULT_SINGLE_ROLES}" | sed 's/ /,/g')
vault write auth/approle/role/${VAULT_AIO_ROLE} \
  token_policies="${COMMAED_ROLES}" \
  token_period=15m \
  ${SECRET_ID_PROPERTIES}
