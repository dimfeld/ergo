INSERT INTO accounts (account_id, account_type_id, name, org_id, fields) VALUES
  (objectid_to_uuid('{{DISCORD_ACCOUNT_ID}}'), 'discord_incoming_webhook', 'Ergo Discord Webhook', objectid_to_uuid('{{ORG_ID}}'), '{"webhook_url": "{{DISCORD_WEBHOOK_URL}}"}')
ON CONFLICT DO NOTHING;
