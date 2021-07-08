BEGIN;
CREATE INDEX inputs_log_task_timestamp_idx ON inputs_log(task_id, updated);
CREATE INDEX actions_log_task_timestamp_idx ON actions_log(task_id, updated);
COMMIT;
