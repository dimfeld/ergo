#!/bin/bash
 set -xeuo pipefail

./load_sql.sh single-user-bootstrap.sql account-types.sql accounts.sql action-categories.sql

