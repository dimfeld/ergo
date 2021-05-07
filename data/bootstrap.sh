set -xeuo pipefile

./load_sql.sh single-user-bootstrap.sql account-types.sql accounts.sql
./load_sql.sh action-categories.sql

./load_input.sh inputs/*.json

