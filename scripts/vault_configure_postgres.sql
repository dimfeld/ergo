-- Commands used to create a PostgreSQL user for Vault.
-- In prodcuction, you should use Vault's password rotation to change the password
-- to something that only it knows.
DO $$BEGIN
  CREATE USER vaultuser WITH PASSWORD 'vaultuser' CREATEROLE;
  EXCEPTION WHEN duplicate_object THEN NULL;
END $$;
