BEGIN;
DROP INDEX tasks_external_task_id_idx;
CREATE UNIQUE INDEX tasks_org_id_external_task_id_idx ON tasks(org_id, external_task_id);
COMMIT;
