ALTER TABLE inputs_log DROP COLUMN queue_job_id;
ALTER TABLE inputs_log DROP COLUMN periodic_trigger_id;
ALTER TABLE inputs_log DROP COLUMN scheduled_for;
REVOKE INSERT, DELETE ON inputs_log FROM ergo_web;

DROP TABLE periodic_triggers;
