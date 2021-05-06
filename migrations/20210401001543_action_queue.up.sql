BEGIN;

CREATE TABLE action_queue (
  action_queue_id bigint primary key generated always as identity,
  task_id bigint not null references tasks ON DELETE CASCADE,
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

COMMIT;
