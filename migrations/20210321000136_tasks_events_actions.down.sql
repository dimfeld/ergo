BEGIN;

DROP TABLE task_triggers_log;
DROP TABLE task_triggers;
DROP TABLE actions_log;
DROP TABLE tasks;
DROP TABLE actions;
DROP TYPE action_status;
DROP TYPE action_executor;
DROP TABLE action_categories;
DROP TABLE inputs_log;
DROP TABLE inputs;
DROP TABLE input_categories;

COMMIT;
