BEGIN;
ALTER TABLE actions DROP COLUMN timeout;
ALTER TABLE action_queue DROP COLUMN timeout;
COMMIT;
