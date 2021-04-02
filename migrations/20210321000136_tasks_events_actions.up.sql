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

CREATE TABLE inputs_log (
  inputs_log_id bigint primary key generated always as identity,
  input_id bigint references inputs,
  payload jsonb,
  error jsonb,
  time timestamptz not null default now()
);

GRANT INSERT ON inputs_log TO ergo_enqueuer;
GRANT SELECT, INSERT ON inputs_log TO ergo_backend;
GRANT SELECT ON inputs_log TO ergo_web;

CREATE TYPE action_executor AS ENUM (
  'http', -- Send an HTTP Request
  'nomad', -- Run a Nomad job
  'input', -- Send an input back into the system
  'spawn' -- Spawn a new task instance from a template
);

CREATE TABLE action_categories (
  action_category_id bigint primary key references object_ids(object_id),
  name text not null,
  description text
);

GRANT SELECT, UPDATE, INSERT, DELETE ON action_categories TO ergo_web;
GRANT SELECT ON action_categories TO ergo_backend;

CREATE TABLE actions (
  action_id bigint primary key references object_ids(object_id),
  action_category_id bigint not null references action_categories,
  name text not null,
  description text,
  input_schema jsonb not null,
  executor action_executor not null,
  executor_data jsonb not null
);

GRANT SELECT, UPDATE, DELETE, INSERT ON actions TO ergo_web;
GRANT SELECT ON actions TO ergo_backend;

CREATE TYPE action_status AS ENUM (
  'pending',
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

GRANT SELECT, UPDATE, DELETE, INSERT ON task_triggers TO ergo_web;
GRANT SELECT ON task_triggers TO ergo_backend;
GRANT SELECT, UPDATE(last_payload) ON task_triggers to ergo_enqueuer;

CREATE TABLE actions_log (
  actions_log_id bigint primary key generated always as identity,
  action_id bigint references actions,
  task_id bigint references tasks,
  payload jsonb,
  response jsonb,
  status action_status not null default 'pending',
  time timestamptz not null default now()
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
