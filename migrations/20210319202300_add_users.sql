BEGIN;

CREATE TABLE user_entity_ids (
  user_entity_id int generated always as identity primary key
);

CREATE TABLE object_ids (
  object_id bigint generated always as identity primary key
);

CREATE TYPE permission AS ENUM (
  'trigger_event'
);

CREATE TABLE user_entity_permissions (
  user_entity_id int not null references user_entity_ids (user_entity_id),
  permission_type permission not null,
  permissioned_object bigint,

  primary key(user_entity_id, permission_type, permissioned_object)
);

COMMENT ON TABLE user_entity_permissions IS 'Permissions for users and roles';

CREATE TABLE orgs (
  org_id int not null primary key references user_entity_ids(user_entity_id),
  external_org_id text unique not null,
  name text not null,
  active boolean default true,
  created timestamptz not null default now()
);

CREATE UNIQUE INDEX ON orgs(external_org_id);

CREATE TABLE users (
  user_id int not null primary key references user_entity_ids(user_entity_id),
  external_user_id text unique not null,
  active_org_id bigint not null references orgs(org_id),
  name text not null,
  email text unique not null,
  password text,
  active boolean default true,
  created timestamptz not null default now()
);

CREATE UNIQUE INDEX ON users (external_user_id);
CREATE UNIQUE INDEX ON users (email);

CREATE TABLE roles (
  role_id bigint not null primary key references user_entity_ids(user_entity_id),
  org_id bigint not null references orgs (org_id),
  name text not null,
  created timestamptz not null default now()
);

CREATE TABLE user_roles (
  user_id bigint not null references users,
  role_id bigint not null references roles,
  primary key (user_id, role_id)
);

CREATE TABLE api_keys (
  api_key text not null primary key,
  secret_key_hash text not null,
  user_entity_id int not null references user_entity_ids,
  description text,

  active boolean default true,
  expires timestamptz,
  created timestamptz not null default now()
);

COMMENT ON TABLE api_keys IS 'API keys for users and organizations';

CREATE TABLE api_key_permissions (
  api_key text references api_keys(api_key),
  permission_type permission not null,
  permissioned_object bigint,

  primary key(api_key, permission_type, permissioned_object)
);

COMMENT ON column api_key_permissions.permissioned_object IS 'An object, or NULL to apply to all objects';

COMMIT;
