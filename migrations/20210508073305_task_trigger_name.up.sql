ALTER TABLE task_triggers ADD column task_trigger_local_id text not null;
ALTER TABLE task_triggers ADD column name text not null;
ALTER TABLE task_triggers ADD column description text;

CREATE INDEX ON task_triggers(task_trigger_local_id);

