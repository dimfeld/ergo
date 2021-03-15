#!/bin/bash
set -e

cd $(dirname $(realpath "$0"))

if [ -f ../.env ]; then
  echo '.env exists but this script needs to overwrite it. Please remove it and try again'
  exit 1
fi

DB_NAME=${DB_NAME:=ergo}
DB_SUPERUSER=${DB_SUPERUSER:=postgres}
DB_HOST=${DB_HOST:=/var/run/postgresql}
DB_PORT=${DB_PORT:=5432}

DB_ADMIN_URL=${DB_ADMIN_URL:=postgres://${DB_SUPERUSER}@${DB_HOST}/postgres}
ADMIN_PASSWORD=$(./gen_passwd.sh)

psql -U ergo_admin -d ergo -h  ${DB_HOST} -p ${DB_PORT} -c "ALTER ROLE ergo_admin WITH PASSWORD ${ADMIN_PASSWORD}"
psql -U ergo_admin -d ergo -h ${DB_HOST} -p ${DB_PORT} -f createdb.sql

cat > ../.env <<EOF
ADMIN_PASSWORD=${ADMIN_PASSWORD}
DATABASE_URL=postgres://ergo_admin:${ADMIN_PASSWORD}@${DB_HOST}/ergo
EOF

sqlx database create
