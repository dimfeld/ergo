BEGIN;
DROP INDEX IF EXISTS inputs_log_task_timestamp_idx;
DROP INDEX IF EXISTS actions_log_task_timestamp_idx;
COMMIT;
