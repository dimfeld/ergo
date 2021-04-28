# Software Dependencies

Ergo is developed against PostgreSQL 12 and Redis 6, but will probably work with recent earlier versions as well.
No special extensions are needed.

# Initialize the Environment

First, you need to set up the environment variables so that database migrations will work.
The simplest way is to create a `.env` file in the project workspace. To initialize the database,
only `DATABASE_URL` is needed, and the rest of the information can be added afterward.

```
# Fill this in with the actual connection string for a user that can create databases and roles.
DATABASE_URL=postgresql://postgres@localhost/ergo

DATABASE_ROLE_WEB_PASSWORD=*****
DATABASE_ROLE_BACKEND_PASSWORD=*****
DATABASE_ROLE_ENQUEUER_PASSWORD=*****

# These are only necessary if they are different from these defaults
DATABASE_NAME=ergo
DATABASE_HOST=localhost
DATABASE_PORT=5432
DATABASE_ROLE_WEB_USERNAME=ergo_web
DATABASE_ROLE_BACKEND_USERNAME=ergo_backend
DATABASE_ROLE_ENQUEUER_USERNAME=ergo_enqueuer
REDIS_URL=localhost
```

# Create the Database

1. Install sqlx-cli: `cargo install sqlx-cli`
2. Run `sqlx database setup` to create the database and run all the migrations.

Once this is done, you can run any future database migrations with `sqlx migrate run`.
Once Ergo reaches a semi-stable state I'll be sure to mention if this is needed
when upgrading between releases.

# Set up Vault (Optional)

Ergo can use [Hashicorp Vault](https://www.vaultproject.io/) to manage database credentials.

1. Install Vault. The [Vault tutorial](https://learn.hashicorp.com/tutorials/vault/getting-started-install) explains how to do this in various ways.
2. Configure Vault to work with PostgreSQL. If you haven't already done this, you can run `scripts/vault_configure_postgres.sql` to create a PostgreSQL user
appropriate for Vault to use. If you've created this user on your own, you can set the `VAULT_PGUSER` and `VAULT_PGPASSWORD`
variables in the `.env` file so that the Vault development script will use them.
3. Run `scripts/vault_dev.sh`. This script starts a development instance of Vault and automatically runs `scripts/vault_roles.sh` to configure it.

When working with a production-ready Vault setup, you can configure it for Ergo by unsealing Vault and running `scripts/vault_roles.sh`. This script
should also be run after any software upgrade that adds new PostgreSQL roles, though these are expected to be rare.

Additionally, the `DATABASE_ROLE_*` environment variables are not needed when using Vault, as Ergo gets that information from
the roles that Vault generates dynamically.



