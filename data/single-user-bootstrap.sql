INSERT INTO orgs (org_id, name) VALUES
  ('{{ORG_UUID}}', '{{ORG_NAME}}')
  ON CONFLICT DO NOTHING;

INSERT INTO users (user_id, active_org_id, name, email, password_hash) VALUES
  ('{{USER_UUID}}', '{{ORG_UUID}}', '{{USER_NAME}}', '{{USER_EMAIL}}', '{{PASSWORD_HASH}}')
  ON CONFLICT DO NOTHING;
