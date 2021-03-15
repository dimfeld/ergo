# Create the PostgreSQL database

First, you need to set up the DATABASE_URL environment variable so that database migrations will work.
The simplest way is to create a `.env` file in the project workspace and set it up with the PostgreSQL
database URL:

```
# Fill this in with the actual connection string
DATABASE_URL=postgresql://postgres@localhost/ergo

# These are only necessary if they are different from these defaults
DATABASE_NAME=ergo
DATABASE_HOST=localhost
DATABASE_PORT=5432
```

1. Install sqlx-cli: `cargo install sqlx-cli`
2. Run `sqlx database setup` to create the database and run all the migrations.

Once this is done, you can run any future database migrations with `sqlx migrate run`.
Once Ergo reaches a semi-stable state I'll be sure to mention if this is needed
when upgrading between releases.

# Set up Vault

Ergo uses [Hashicorp Vault](https://www.vaultproject.io/) by default to manage database credentials.

1. Install Vault. The [Vault tutorial](https://learn.hashicorp.com/tutorials/vault/getting-started-install) explains how to do this in various ways.
2. Configure Vault to work with PostgreSQL. If you haven't already done this, you can execute `scripts/vault_configure_postgres.sql` to create a PostgreSQL user
appropriate for Vault to use.
3. Run `scripts/vault_dev.sh`. This script starts a development instance of Vault and automatically runs `scripts/vault_roles.sh` to configure it.

When working with a production-ready Vault setup, you can configure it for Ergo by unsealing Vault and running `scripts/vault_roles.sh`. This script can also be run after
any software upgrade that adds new PostgreSQL roles, though these are expected to be rare.

At some point in the future I'll add support for running without Vault.

