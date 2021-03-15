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

