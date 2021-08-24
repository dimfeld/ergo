BEGIN;

ALTER TABLE tasks ADD COLUMN state_machine_config jsonb;
UPDATE tasks SET state_machine_config=compiled
FROM task_templates
WHERE tasks.task_template_id=task_templates.task_template_id
  AND tasks.task_template_version=task_templates.task_template_version;

ALTER TABLE tasks DROP CONSTRAINT task_template_constraint;
ALTER TABLE tasks DROP COLUMN task_template_id;
ALTER TABLE tasks DROP COLUMN task_template_version;
ALTER TABLE tasks RENAME COLUMN state TO state_machine_states;

DROP TABLE task_templates;

COMMIT;
