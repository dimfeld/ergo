INSERT INTO user_entity_ids (user_entity_id) VALUES
  ('{{ORG_ID}}'),
  ('{{USER_ID}}')
  ON CONFLICT DO NOTHING;

INSERT INTO orgs (org_id, name) VALUES
  ('{{ORG_ID}}', '{{ORG_NAME}}')
  ON CONFLICT DO NOTHING;

INSERT INTO users (user_id, active_org_id, name, email, password_hash) VALUES
  ('{{USER_ID}}', '{{ORG_ID}}', '{{USER_NAME}}', '{{USER_EMAIL}}', '{{PASSWORD_HASH}}')
  ON CONFLICT DO NOTHING;
