BEGIN;

CREATE TABLE input_categories (
  input_category_id bigint primary key references object_ids(object_id),
  name text not null,
  description text
);

GRANT SELECT, UPDATE, INSERT, DELETE ON input_categories TO ergo_web;
GRANT SELECT ON input_categories TO ergo_backend;

CREATE TABLE inputs (
  input_id bigint primary key references object_ids(object_id),
  input_category_id bigint references input_categories(input_category_id),
  name text not null,
  description text,
  payload_schema jsonb not null
);

GRANT SELECT ON inputs TO ergo_enqueuer;
GRANT SELECT ON inputs TO ergo_backend;
GRANT SELECT, UPDATE, INSERT, DELETE ON inputs TO ergo_web;

CREATE TYPE input_status AS ENUM (
  'pending',
  'success',
  'error'
);

CREATE TABLE inputs_log (
  inputs_log_id uuid primary key,
  task_trigger_id bigint references inputs,
  status input_status not null default 'pending',
  payload jsonb,
  error jsonb,
  created timestamptz not null default now(),
  updated timestamptz not null default now()
);

GRANT INSERT ON inputs_log TO ergo_enqueuer;
GRANT SELECT, INSERT ON inputs_log TO ergo_backend;
GRANT SELECT ON inputs_log TO ergo_web;

CREATE TABLE action_categories (
  action_category_id bigint primary key references object_ids(object_id),
  name text not null,
  description text
);

GRANT SELECT, UPDATE, INSERT, DELETE ON action_categories TO ergo_web;
GRANT SELECT ON action_categories TO ergo_backend;

CREATE TABLE account_types (
  account_type_id text primary key,
  name text not null,
  description text,
  fields text[]
);

GRANT SELECT ON account_types TO ergo_backend;
GRANT SELECT ON account_types TO ergo_web;

CREATE TABLE accounts (
  account_id bigint primary key references object_ids(object_id),
  account_type_id text not null references account_types,
  name text not null,
  org_id uuid not null references orgs,
  user_id uuid references users,
  fields jsonb,
  expires timestamptz
);

GRANT SELECT ON accounts to ergo_backend;
-- Allow ergo_web to set account secrets but not to read them.
GRANT SELECT(account_id, name, org_id, user_id, expires), UPDATE, INSERT, DELETE ON accounts to ergo_web;

CREATE TABLE actions (
  action_id bigint primary key references object_ids(object_id),
  action_category_id bigint not null references action_categories,
  name text not null,
  description text,
  executor_id text not null,
  executor_template jsonb not null,
  template_fields jsonb not null,
  account_required boolean not null default false
);

GRANT SELECT, UPDATE, DELETE, INSERT ON actions TO ergo_web;
GRANT SELECT ON actions TO ergo_backend;

CREATE TABLE allowed_action_account_types (
  account_type_id text references account_types ON DELETE CASCADE,
  action_id bigint references actions ON DELETE CASCADE,
  PRIMARY KEY(account_type_id, action_id)
);
COMMENT ON TABLE allowed_action_account_types IS 'The types of accounts that are allowed to be linked to an action';

GRANT SELECT ON allowed_action_account_types TO ergo_backend;
GRANT SELECT ON allowed_action_account_types TO ergo_web;

CREATE TYPE action_status AS ENUM (
  'pending',
  'running',
  'success',
  'error'
);

CREATE TABLE tasks (
  task_id bigint primary key references object_ids(object_id),
  external_task_id text not null,
  org_id uuid not null references orgs(org_id),
  name text not null,
  description text,
  enabled boolean not null default false,
  state_machine_config jsonb not null,
  state_machine_states jsonb not null,
  created timestamptz not null default now(),
  modified timestamptz not null default now()
);

CREATE UNIQUE INDEX ON tasks(external_task_id);

GRANT SELECT, UPDATE, DELETE, INSERT ON tasks TO ergo_web;
GRANT SELECT, UPDATE, DELETE, INSERT ON tasks TO ergo_backend;
GRANT SELECT ON tasks to ergo_enqueuer;

CREATE TABLE task_triggers (
  task_trigger_id bigint primary key references object_ids(object_id),
  task_id bigint not null references tasks(task_id),
  input_id bigint not null references inputs(input_id),
  last_payload jsonb
);

CREATE INDEX task_triggers_input_id ON task_triggers (input_id);
CREATE INDEX task_triggers_task_id ON task_triggers (task_id);

GRANT SELECT, UPDATE, DELETE, INSERT ON task_triggers TO ergo_web;
GRANT SELECT ON task_triggers TO ergo_backend;
GRANT SELECT, UPDATE(last_payload) ON task_triggers to ergo_enqueuer;

CREATE TABLE task_actions (
  task_id bigint not null references tasks,
  task_action_local_id text not null,
  action_id bigint not null references actions,
  account_id bigint references accounts,
  name text not null,
  action_template jsonb,
  PRIMARY KEY(task_id, task_action_local_id)
);

CREATE INDEX task_actions_task_id ON task_actions(task_id);
COMMENT ON COLUMN task_actions.task_action_local_id IS 'The ID of the task action within the task';

CREATE TABLE actions_log (
  actions_log_id uuid primary key,
  inputs_log_id uuid references inputs_log,
  task_id bigint not null,
  task_action_local_id text,
  payload jsonb,
  result jsonb,
  status action_status not null default 'pending',
  created timestamptz not null default now(),
  updated timestamptz not null default now()
);

GRANT SELECT ON actions_log TO ergo_web;
GRANT SELECT, INSERT, UPDATE ON actions_log TO ergo_backend;

CREATE TABLE task_triggers_log (
  task_triggers_log_id bigint primary key generated always as identity,
  task_trigger_id bigint references task_triggers,
  payload jsonb,
  time timestamptz not null default now()
);

GRANT SELECT ON task_triggers_log TO ergo_web;
GRANT SELECT, INSERT ON task_triggers_log TO ergo_backend;

COMMIT;
