#!/bin/bash
set -euo pipefail

cd $(realpath $(dirname $0))
source ./env.sh

postgresql_autodoc -t dot -d ${DATABASE_NAME} -u ergo_admin -h ${DATABASE_HOST} -p ${DATABASE_PORT} -w
dot -Tsvg ${DATABASE_NAME}.dot -o ../db-schema.svg
rm -f ${DATABASE_NAME}.dot
