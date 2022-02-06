REVOKE SELECT ON accounts from ergo_web;
GRANT SELECT(account_id, name, org_id, user_id, expires), UPDATE, INSERT, DELETE ON accounts to ergo_web;
