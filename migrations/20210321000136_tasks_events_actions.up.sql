BEGIN;

create or replace function objectid_to_uuid(text)
returns uuid
as $$
select
  encode(decode(
    replace(replace(right($1, 22), '-', '+'), '_', '/') || '==',
  'base64'), 'hex')::uuid
$$
language sql
immutable
returns null on null input
parallel safe;

-- Don't grant access to normal uses to discourage use by normal code which should just use the Rust serialization.
COMMENT ON FUNCTION objectid_to_uuid IS 'A utility function to convert a text object ID to the underlying UUID.';

CREATE TABLE input_categories (
  input_category_id uuid primary key,
  name text not null,
  description text
);

GRANT SELECT, UPDATE, INSERT, DELETE ON input_categories TO ergo_web;
GRANT SELECT ON input_categories TO ergo_backend;

CREATE TABLE inputs (
  input_id uuid primary key,
  input_category_id uuid references input_categories(input_category_id),
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
  task_trigger_id uuid,
  task_id uuid,
  task_trigger_local_id text not null,
  status input_status not null default 'pending',
  payload jsonb,
  error jsonb,
  created timestamptz not null default now(),
  updated timestamptz not null default now()
);

GRANT INSERT ON inputs_log TO ergo_enqueuer;
GRANT SELECT, INSERT, UPDATE ON inputs_log TO ergo_backend;
GRANT SELECT ON inputs_log TO ergo_web;

CREATE TABLE action_categories (
  action_category_id uuid primary key,
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
  account_id uuid primary key,
  account_type_id text not null references account_types,
  name text not null,
  org_id uuid not null references orgs ON DELETE CASCADE,
  user_id uuid references users ON DELETE CASCADE,
  fields jsonb,
  expires timestamptz
);

GRANT SELECT ON accounts to ergo_backend;
-- Allow ergo_web to set account secrets but not to read them.
GRANT SELECT(account_id, name, org_id, user_id, expires), UPDATE, INSERT, DELETE ON accounts to ergo_web;

CREATE TABLE actions (
  action_id uuid primary key,
  action_category_id uuid not null references action_categories,
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
  action_id uuid references actions ON DELETE CASCADE,
  PRIMARY KEY(account_type_id, action_id)
);
COMMENT ON TABLE allowed_action_account_types IS 'The types of accounts that are allowed to be linked to an action';

GRANT SELECT ON allowed_action_account_types TO ergo_backend;
GRANT SELECT, INSERT, UPDATE, DELETE ON allowed_action_account_types TO ergo_web;

CREATE TYPE action_status AS ENUM (
  'pending',
  'running',
  'success',
  'error'
);

CREATE TABLE tasks (
  task_id uuid primary key,
  org_id uuid not null references orgs(org_id),
  name text not null,
  description text,
  enabled boolean not null default false,
  deleted boolean not null default false,
  state_machine_config jsonb not null,
  state_machine_states jsonb not null,
  created timestamptz not null default now(),
  modified timestamptz not null default now()
);

CREATE UNIQUE INDEX ON tasks(org_id, task_id) WHERE NOT deleted;

GRANT SELECT, UPDATE, DELETE, INSERT ON tasks TO ergo_web;
GRANT SELECT, UPDATE, DELETE, INSERT ON tasks TO ergo_backend;
GRANT SELECT ON tasks to ergo_enqueuer;

CREATE TABLE task_triggers (
  task_trigger_id uuid primary key,
  task_id uuid not null references tasks(task_id) ON DELETE CASCADE,
  task_trigger_local_id text not null,
  input_id uuid not null references inputs(input_id),
  name text not null,
  description text,
  last_payload jsonb
);

CREATE INDEX task_triggers_input_id ON task_triggers (input_id);
CREATE INDEX task_triggers_task_id ON task_triggers (task_id);

GRANT SELECT, UPDATE, DELETE, INSERT ON task_triggers TO ergo_web;
GRANT SELECT ON task_triggers TO ergo_backend;
GRANT SELECT, UPDATE(last_payload) ON task_triggers to ergo_enqueuer;

CREATE TABLE task_actions (
  task_id uuid not null references tasks ON DELETE CASCADE,
  task_action_local_id text not null,
  action_id uuid not null references actions,
  account_id uuid references accounts,
  name text not null,
  action_template jsonb,
  PRIMARY KEY(task_id, task_action_local_id)
);

GRANT SELECT ON task_actions TO ergo_backend;
GRANT SELECT ON task_actions TO ergo_enqueuer;
GRANT SELECT, UPDATE, DELETE, INSERT ON task_actions TO ergo_web;

CREATE INDEX task_actions_task_id ON task_actions(task_id);
COMMENT ON COLUMN task_actions.task_action_local_id IS 'The ID of the task action within the task';

CREATE TABLE actions_log (
  actions_log_id uuid primary key,
  inputs_log_id uuid references inputs_log ON DELETE SET NULL,
  task_id uuid not null,
  task_action_local_id text,
  payload jsonb,
  result jsonb,
  status action_status not null default 'pending',
  created timestamptz not null default now(),
  updated timestamptz not null default now()
);

CREATE INDEX ON actions_log(inputs_log_id);
CREATE INDEX ON actions_log(task_id);

GRANT SELECT ON actions_log TO ergo_web;
GRANT SELECT, INSERT, UPDATE ON actions_log TO ergo_backend;

CREATE TABLE event_queue (
  event_queue_id bigint primary key generated always as identity,
  task_id uuid not null references tasks ON DELETE CASCADE,
  task_trigger_id uuid not null references task_triggers ON DELETE CASCADE,
  input_id uuid not null references inputs ON DELETE CASCADE,
  inputs_log_id uuid not null,
  payload jsonb,
  time timestamptz not null default now()
);

GRANT SELECT, INSERT, DELETE, UPDATE ON event_queue TO ergo_web;
GRANT SELECT, INSERT, DELETE, UPDATE ON event_queue TO ergo_backend;
GRANT INSERT ON event_queue TO ergo_enqueuer;

CREATE TABLE action_queue (
  action_queue_id bigint primary key generated always as identity,
  task_id uuid not null references tasks ON DELETE CASCADE,
  task_action_local_id text not null,
  actions_log_id uuid not null,
  input_arrival_id uuid,
  payload jsonb,
  time timestamptz not null default now(),
  FOREIGN KEY (task_id, task_action_local_id) REFERENCES task_actions
);

GRANT SELECT, INSERT, DELETE, UPDATE ON action_queue TO ergo_web;
GRANT SELECT, INSERT, DELETE, UPDATE ON action_queue TO ergo_backend;
GRANT INSERT ON action_queue TO ergo_enqueuer;

CREATE TYPE notify_service AS ENUM (
  'email',
  'discord_incoming_webhook',
  'slack_incoming_webhook'
);

CREATE TABLE notify_endpoints (
  notify_endpoint_id uuid primary key default uuid_generate_v4(),
  org_id uuid not null references orgs,
  service notify_service not null,
  destination text not null,
  enabled bool not null default true
);

CREATE TYPE notify_event AS ENUM (
  'input_arrived',
  'input_processed',
  'action_started',
  'action_success',
  'action_error'
);

CREATE TABLE notify_listeners (
  notify_listener_id uuid primary key default uuid_generate_v4(),
  notify_endpoint_id uuid not null references notify_endpoints,
  object_id uuid not null,
  event notify_event not null,
  org_id uuid not null references orgs ON DELETE CASCADE,
  enabled bool not null default true
);

COMMENT ON COLUMN notify_listeners.object_id is 'The object to listen on, or the nil UUID for all applicable objects';

CREATE INDEX notify_listeners_event_object_id ON notify_listeners(event, object_id);

COMMIT;
