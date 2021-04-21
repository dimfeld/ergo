BEGIN;

CREATE TABLE action_queue (
  action_queue_id bigint primary key generated always as identity,
  task_id bigint not null references tasks ON DELETE CASCADE,
  task_trigger_id bigint references task_triggers ON DELETE CASCADE,
  task_action_id bigint not null references task_actions ON DELETE CASCADE,
  payload jsonb,
  time timestamptz not null default now()
);

GRANT SELECT, INSERT, DELETE, UPDATE ON action_queue TO ergo_web;
GRANT SELECT, INSERT, DELETE, UPDATE ON action_queue TO ergo_backend;
GRANT INSERT ON action_queue TO ergo_enqueuer;

COMMIT;
