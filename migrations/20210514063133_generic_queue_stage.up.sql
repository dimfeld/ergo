CREATE TABLE queue_stage (
  id bigint primary key generated always as identity,
  queue text not null,
  job_id text not null,
  payload jsonb not null,
  timeout int,
  max_retries int,
  run_at timestamptz,
  retry_backoff int,
  time timestamptz not null default now()
);

GRANT SELECT, INSERT, DELETE, UPDATE ON queue_stage TO ergo_web;
GRANT SELECT, INSERT, DELETE, UPDATE ON queue_stage TO ergo_backend;
GRANT INSERT ON queue_stage TO ergo_enqueuer;
