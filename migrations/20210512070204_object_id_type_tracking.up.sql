BEGIN;
CREATE TYPE object_type AS ENUM (
  'account',
  'action_category',
  'action',
  'global',
  'input_category',
  'input',
  'notify_listener',
  'task_trigger',
  'task'
);

ALTER TABLE object_ids ADD COLUMN type object_type;

WITH all_types AS (
  SELECT 1 as id, 'global'::object_type AS obj_type
  UNION ALL
  SELECT account_id AS id, 'account'::object_type as obj_type
  FROM accounts
  UNION ALL
  SELECT action_category_id AS id, 'action_category'::object_type
  FROM action_categories
  UNION ALL
  SELECT action_id AS id, 'action'::object_type
  FROM actions
  UNION ALL
  SELECT input_category_id AS id, 'input_category'::object_type
  FROM input_categories
  UNION ALL
  SELECT input_id AS id, 'input'::object_type
  FROM inputs
  UNION ALL
  SELECT object_id AS id, 'notify_listener'::object_type
  FROM notify_listeners
  UNION ALL
  SELECT task_trigger_id AS id, 'task_trigger'::object_type
  FROM task_triggers
  UNION ALL
  SELECT task_id AS id, 'task'::object_type
  FROM tasks
)
UPDATE object_ids
SET type=obj_type
FROM all_types
WHERE all_types.id=object_ids.object_id;

ALTER TABLE object_ids ALTER COLUMN type SET NOT NULL;

COMMIT;
