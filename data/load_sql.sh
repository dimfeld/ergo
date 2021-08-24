#!/bin/bash
set -euo pipefail
source ../.env

export ORG_UUID=$(cargo run id to-uuid ${ORG_ID})
export USER_UUID=$(cargo run id to-uuid ${USER_ID})

for file in "$@"; do
  echo $file
  env-template "$file" | psql -d $DATABASE_URL -1
done
