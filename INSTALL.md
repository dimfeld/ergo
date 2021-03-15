# Set up Vault

Ergo uses [Hashicorp Vault](https://www.vaultproject.io/) by default to manage database credentials.

1. Install Vault. The [Vault tutorial](https://learn.hashicorp.com/tutorials/vault/getting-started-install) explains how to do this in various ways.
2. Configure Vault to work with PostgreSQL. If you haven't already done this, you can execute `scripts/vault_configure_postgres.sql` to create a PostgreSQL user
appropriate for Vault to use.
3. Run `scripts/vault_dev.sh`. This script starts a development instance of Vault and automatically runs `scripts/vault_roles.sh` to configure it.

When working with a production-ready Vault setup, you can configure it for Ergo by unsealing Vault and running `scripts/vault_roles.sh`. This script can also be run after
any software upgrade that adds new PostgreSQL roles, though these are expected to be rare.

# Create the PostgreSQL database

Log in as your Postgres super user and run this:

```
CREATE DATABASE ergo;
CREATE USER ergo_admin;
GRANT ALL ON DATABASE ergo to ergo_admin;
```

The `ergo_admin` user is used to run migrations and otherwise interact with the database during development and deployment.
All other role management goes through the Vault configuration.

