BEGIN;
CREATE TABLE action_queue (
    action_queue_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    task_id uuid NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    task_action_local_id text NOT NULL,
    actions_log_id uuid NOT NULL,
    input_arrival_id uuid,
    payload jsonb,
    time timestamp with time zone NOT NULL DEFAULT now(),
    timeout integer,
    CONSTRAINT action_queue_task_id_task_action_local_id_fkey FOREIGN KEY (task_id, task_action_local_id) REFERENCES task_actions(task_id, task_action_local_id)
);

CREATE TABLE event_queue (
    event_queue_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    task_id uuid NOT NULL REFERENCES tasks(task_id) ON DELETE CASCADE,
    task_trigger_id uuid NOT NULL REFERENCES task_triggers(task_trigger_id) ON DELETE CASCADE,
    input_id uuid NOT NULL REFERENCES inputs(input_id) ON DELETE CASCADE,
    inputs_log_id uuid NOT NULL,
    payload jsonb,
    time timestamp with time zone NOT NULL DEFAULT now()
);

COMMIT;
