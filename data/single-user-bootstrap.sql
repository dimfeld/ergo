INSERT INTO orgs (org_id, name) VALUES
  (objectid_to_uuid('{{ORG_ID}}'), '{{ORG_NAME}}')
  ON CONFLICT DO NOTHING;

INSERT INTO users (user_id, active_org_id, name, email, password_hash) VALUES
  (objectid_to_uuid('{{USER_ID}}'), objectid_to_uuid('{{ORG_ID}}'), '{{USER_NAME}}', '{{USER_EMAIL}}', '{{PASSWORD_HASH}}')
  ON CONFLICT DO NOTHING;
