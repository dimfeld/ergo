#!/bin/bash
set -euo pipefail

[ -f ../.env ] && source ../.env

for file in "$@"; do
  echo $file
  object=$(env-template "$file" | json5)
  id=$(jq -r .input_id <<< "$object")
  http PUT ${HOST:-http://localhost:6543}/api/inputs/$id\?api_key=${API_KEY} <<<"$object"
done
