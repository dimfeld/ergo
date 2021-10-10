ALTER TABLE queue_stage ALTER COLUMN payload SET NOT NULL;
ALTER TABLE queue_stage DROP COLUMN operation;
