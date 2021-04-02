#!/bin/bash
set -euo pipefail

if [ -f ../.env ]; then
  set -o allexport
  source ../.env
  set +o allexport
fi

export ENV=${ENV:=dev}
export VAULT_SINGLE_ROLES="ergo_web ergo_backend ergo_enqueuer"
export VAULT_AIO_ROLE="ergo_aio_server"
export VAULT_TOKEN_PERIOD=${VAULT_TOKEN_PERIOD:=15m}

export DATABASE_NAME=${DATABASE_NAME:=ergo}
export DATABASE_HOST=${DATABASE_HOST:=localhost}
export DATABASE_PORT=${DATABASE_PORT:=5432}

export VAULT_ADDR=${VAULT_ADDR:=http://127.0.0.1:8200}
export VAULT_TOKEN=${VAULT_TOKEN:=root}
export VAULT_DATABASE_ROLE_TTL=${VAULT_DATABASE_ROLE_TTL:=1h}
export VAULT_DATABASE_ROLE_MAX_TTL=${VAULT_DATABASE_ROLE_MAX_TTL:=8h}
