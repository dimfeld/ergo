BEGIN;
ALTER TABLE actions ADD COLUMN timeout int;
ALTER TABLE action_queue ADD COLUMN timeout int;
COMMIT;
