BEGIN;

CREATE TABLE task_templates (
  task_template_id uuid,
  task_template_version bigint,
  org_id uuid not null references orgs(org_id),
  name text not null,
  description text,
  source jsonb not null,
  compiled jsonb not null,
  initial_state jsonb not null,
  created timestamptz default now(),
  modified timestamptz default now(),
  PRIMARY KEY(task_template_id, task_template_version)
);

CREATE TEMPORARY TABLE task_template_transfer AS
  SELECT task_id, uuid_generate_v1mc() AS task_template_id from tasks;

INSERT INTO task_templates
  (task_template_id, task_template_version, org_id, name, description,
    source, compiled, initial_state, created, modified)
SELECT task_template_id, 0, org_id, name, description,
  json_build_object('type', 'StateMachine', 'data', state_machine_config),
  json_build_object('type', 'StateMachine', 'data', state_machine_config),
  state_machine_states,
  created, modified
FROM task_template_transfer
JOIN tasks USING(task_id);

ALTER TABLE tasks DROP COLUMN state_machine_config;
ALTER TABLE tasks ADD COLUMN task_template_id uuid;
ALTER TABLE tasks ADD COLUMN task_template_version bigint;
UPDATE tasks SET task_template_version=0;

ALTER TABLE tasks RENAME COLUMN state_machine_states TO state;

UPDATE tasks SET task_template_id=t.task_template_id
FROM (SELECT task_id, task_template_id FROM task_template_transfer) t
WHERE tasks.task_id=t.task_id;

ALTER TABLE tasks ALTER COLUMN task_template_id SET NOT NULL;
ALTER TABLE tasks ALTER COLUMN task_template_version SET NOT NULL;
ALTER TABLE tasks ADD CONSTRAINT task_template_constraint
  FOREIGN KEY (task_template_id, task_template_version) REFERENCES task_templates;

COMMIT;
