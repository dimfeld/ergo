BEGIN;

DROP TABLE inputs;
DROP TABLE inputs_log;
DROP TYPE action_executor;
DROP TABLE action_categories;
DROP TABLE actions;
DROP TYPE action_status;
DROP TABLE actions_log;
DROP TABLE tasks;
DROP TABLE task_triggers;
DROP TABLE task_triggers_log;

COMMIT;
