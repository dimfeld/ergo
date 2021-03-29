BEGIN;

CREATE TABLE event_queue (
  event_queue_id bigint primary key generated always as identity,
  task_id bigint references tasks ON DELETE CASCADE,
  task_trigger_id bigint references task_triggers ON DELETE CASCADE,
  input_id bigint references inputs ON DELETE CASCADE,
  payload jsonb,
  time timestamptz not null default now()
);

COMMIT;
