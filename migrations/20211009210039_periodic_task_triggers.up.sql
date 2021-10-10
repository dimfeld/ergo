CREATE TABLE periodic_triggers (
  periodic_trigger_id uuid primary key,
  task_trigger_id uuid not null references task_triggers ON DELETE CASCADE,
  name text,
  schedule jsonb not null,
  payload jsonb not null,
  enabled boolean not null default true
);

GRANT SELECT, INSERT, DELETE, UPDATE ON periodic_triggers TO ergo_web;
GRANT SELECT ON periodic_triggers TO ergo_backend;
GRANT SELECT ON periodic_triggers TO ergo_enqueuer;

CREATE INDEX ON periodic_triggers (task_trigger_id);

ALTER TABLE inputs_log ADD COLUMN queue_job_id text;
UPDATE inputs_log SET queue_job_id='';
ALTER TABLE inputs_log ALTER COLUMN queue_job_id SET NOT NULL;
ALTER TABLE inputs_log ADD COLUMN periodic_trigger_id uuid;
ALTER TABLE inputs_log ADD COLUMN scheduled_for timestamptz;

CREATE INDEX ON inputs_log (periodic_trigger_id) WHERE status = 'pending' AND periodic_trigger_id IS NOT NULL;

