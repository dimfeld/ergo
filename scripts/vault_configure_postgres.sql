-- Commands used to create a PostgreSQL user for Vault.
-- This password will be rotated by Vault to something that only it knows.
CREATE USER IF NOT EXIST vault WITH PASSWORD 'vaultuser' CREATEROLE;
