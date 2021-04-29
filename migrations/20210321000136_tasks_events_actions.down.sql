BEGIN;

DROP TABLE actions_log;
DROP TABLE task_triggers_log;
DROP TABLE task_triggers;
DROP TABLE task_actions;
DROP TABLE tasks;
DROP TABLE allowed_action_account_types;
DROP TABLE actions;
DROP TYPE action_status;
DROP TABLE action_categories;
DROP TABLE executors;
DROP TABLE inputs_log;
DROP TYPE input_status;
DROP TABLE inputs;
DROP TABLE input_categories;
DROP TABLE accounts;
DROP TABLE account_types;

COMMIT;
