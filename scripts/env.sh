#!/bin/bash
set -euo pipefail

if [ -f ../.env ]; then
  set -o allexport
  source ../.env
  set +o allexport
fi

export DATABASE_NAME=${DATABASE_NAME:=ergo}
export DATABASE_HOST=${DATABASE_HOST:=localhost}
export DATABASE_PORT=${DATABASE_PORT:=5432}

export VAULT_ADDR=${VAULT_ADDR:=http://127.0.0.1:8200}
export VAULT_TOKEN=${VAULT_TOKEN:=root}

