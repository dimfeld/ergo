CREATE TABLE action_queue (
  action_queue_id bigint primary key generated always as identity,
  task_id bigint not null references tasks ON DELETE CASCADE,
  task_trigger_id bigint references task_triggers ON DELETE CASCADE,
  action_id bigint not null references actions ON DELETE CASCADE,
  payload jsonb,
  time timestamptz not null default now()
);
