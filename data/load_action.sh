#!/bin/bash
set -euo pipefail

[ -f ../.env ] && source ../.env

for file in "$@"; do
  echo $file
  env-template "$file" | http POST ${HOST:-http://localhost:6543}/api/actions\?api_key=${API_KEY}
done
