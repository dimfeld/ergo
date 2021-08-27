INSERT INTO action_categories (action_category_id, name) VALUES
  (objectid_to_uuid('{{ACTION_CATEGORY_ID_GENERAL}}'), 'General')
ON CONFLICT DO NOTHING;
