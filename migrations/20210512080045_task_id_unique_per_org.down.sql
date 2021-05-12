DROP INDEX tasks_org_id_external_task_id_idx;
CREATE UNIQUE INDEX ON tasks(external_task_id);
