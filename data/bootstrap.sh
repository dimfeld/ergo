set -xeuo pipefile

# If you don't have env-template, you can get it via `cargo install env-template`.
env-template single-user-bootstrap.sql | psql -1 -d ${DATABASE_URL}
env-template account-types.sql | psql -1 -d ${DATABASE_URL}
env-template accounts.sql | psql -1 -d ${DATABASE_URL}

