BEGIN;

CREATE TABLE event_queue (
  event_queue_id bigint primary key generated always as identity,
  task_id bigint not null references tasks ON DELETE CASCADE,
  task_trigger_id bigint not null references task_triggers ON DELETE CASCADE,
  input_id bigint not null references inputs ON DELETE CASCADE,
  inputs_log_id uuid not null,
  payload jsonb,
  time timestamptz not null default now()
);

GRANT SELECT, INSERT, DELETE, UPDATE ON event_queue TO ergo_web;
GRANT SELECT, INSERT, DELETE, UPDATE ON event_queue TO ergo_backend;
GRANT INSERT ON event_queue TO ergo_enqueuer;

COMMIT;
