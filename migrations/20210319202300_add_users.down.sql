BEGIN;

DROP TABLE api_key_permissions;
DROP TABLE api_keys;
DROP TABLE user_roles;
DROP TABLE roles;
DROP TABLE users;
DROP TABLE orgs;
DROP TABLE user_entity_permissions;
DROP TYPE permission;
DROP TABLE object_ids;
DROP TABLE user_entity_ids;

COMMIT;
