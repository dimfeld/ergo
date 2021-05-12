#!/bin/bash
set -euo pipefail

[ -f ../.env ] && source ../.env

for file in "$@"; do
  echo $file
  object=$(env-template "$file")
  id=$(jq -r .task_id <<< "$object")
  env-template "$file" | http PUT ${HOST:-http://localhost:6543}/api/tasks/$id\?api_key=${API_KEY}
done
