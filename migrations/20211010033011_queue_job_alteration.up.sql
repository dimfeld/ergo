ALTER TABLE queue_stage ADD COLUMN operation text default 'add';
ALTER TABLE queue_stage ALTER COLUMN payload DROP NOT NULL;
