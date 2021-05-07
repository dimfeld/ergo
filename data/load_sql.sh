#!/bin/bash
set -euo pipefail
source ../.env

for file in "$@"; do
  echo $file
  env-template "$file" | psql -d $DATABASE_URL -1
done
