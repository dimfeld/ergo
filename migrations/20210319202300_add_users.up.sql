BEGIN;

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

GRANT EXECUTE ON FUNCTION uuid_nil TO ergo_user;
GRANT EXECUTE ON FUNCTION uuid_generate_v4 TO ergo_user;

CREATE TYPE permission AS ENUM (
  'read',
  'write',
  'create',
  'trigger_event'
);

CREATE TABLE user_entity_permissions (
  user_entity_id uuid not null,
  permission_type permission not null,
  permissioned_object uuid not null,

  primary key(user_entity_id, permission_type, permissioned_object)
);

COMMENT ON TABLE user_entity_permissions IS 'Permissions for users and roles';

GRANT SELECT ON user_entity_permissions TO ergo_enqueuer;
GRANT SELECT ON user_entity_permissions TO ergo_backend;
GRANT SELECT, UPDATE, DELETE, INSERT ON user_entity_permissions TO ergo_web;

CREATE TABLE orgs (
  org_id uuid not null primary key,
  name text not null,
  deleted boolean default false,
  created timestamptz not null default now()
);

GRANT SELECT(org_id, deleted) ON orgs TO ergo_enqueuer;
GRANT SELECT(org_id, deleted) ON orgs TO ergo_backend;
GRANT SELECT, UPDATE, DELETE, INSERT ON orgs TO ergo_web;

CREATE TABLE users (
  user_id uuid not null primary key,
  active_org_id uuid not null references orgs(org_id),
  name text not null,
  email text not null,
  password_hash text,
  deleted boolean default false,
  created timestamptz not null default now()
);

CREATE UNIQUE INDEX ON users (email) WHERE NOT deleted;

GRANT SELECT(user_id, active_org_id, deleted, name, email) ON users TO ergo_enqueuer;
GRANT SELECT(user_id, active_org_id, deleted, name, email) ON users TO ergo_backend;
GRANT SELECT, UPDATE, DELETE, INSERT ON users TO ergo_web;

CREATE TABLE roles (
  role_id uuid not null primary key,
  org_id uuid not null references orgs (org_id),
  name text not null,
  created timestamptz not null default now()
);

GRANT SELECT ON roles TO ergo_enqueuer;
GRANT SELECT ON roles TO ergo_backend;
GRANT SELECT, UPDATE, DELETE, INSERT ON roles TO ergo_web;

CREATE TABLE user_roles (
  user_id uuid not null references users,
  role_id uuid not null references roles,
  org_id uuid not null references orgs,
  primary key (user_id, org_id, role_id)
);

GRANT SELECT ON user_roles TO ergo_enqueuer;
GRANT SELECT ON user_roles TO ergo_backend;
GRANT SELECT, UPDATE, DELETE, INSERT ON user_roles TO ergo_web;

CREATE TABLE api_keys (
  api_key_id uuid primary key,
  prefix text not null,
  hash bytea not null,
  org_id uuid not null references orgs,
  user_id uuid references users,
  inherits_user_permissions bool not null default false,
  description text,

  active boolean not null default true,
  expires timestamptz,
  created timestamptz not null default now()
);

CREATE INDEX ON api_keys (org_id);

COMMENT ON TABLE api_keys IS 'API keys for users and organizations';

GRANT SELECT ON api_keys TO ergo_enqueuer;
GRANT SELECT, UPDATE(active) ON api_keys TO ergo_backend;
GRANT SELECT, UPDATE, DELETE, INSERT ON api_keys TO ergo_web;

COMMIT;
