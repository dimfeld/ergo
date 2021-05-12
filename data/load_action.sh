#!/bin/bash
set -euo pipefail

[ -f ../.env ] && source ../.env

for file in "$@"; do
  echo $file
  object=$(env-template "$file")
  id=$(jq -r .action_id <<< "$object")
  http PUT ${HOST:-http://localhost:6543}/api/actions/$id\?api_key=${API_KEY} <<<"$object"
done
