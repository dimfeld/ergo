#!/bin/bash
set -euo pipefail

[ -f ../.env ] && source ../.env

for file in "$@"; do
  echo $file
  object=$(env-template "$file" | json5)
  env-template "$file" | http POST ${HOST:-http://localhost:6543}/api/tasks\?api_key=${API_KEY}
done
