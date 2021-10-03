-- Add up migration script here
BEGIN;
DROP TABLE action_queue;
DROP TABLE event_queue;
COMMIT;
