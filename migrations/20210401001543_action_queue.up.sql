BEGIN;

CREATE TABLE action_queue (
  action_queue_id bigint primary key generated always as identity,
  task_action_id bigint not null references task_actions ON DELETE CASCADE,
  input_arrival_id uuid,
  payload jsonb,
  time timestamptz not null default now()
);

GRANT SELECT, INSERT, DELETE, UPDATE ON action_queue TO ergo_web;
GRANT SELECT, INSERT, DELETE, UPDATE ON action_queue TO ergo_backend;
GRANT INSERT ON action_queue TO ergo_enqueuer;

COMMIT;
