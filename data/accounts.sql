INSERT INTO object_ids (object_id) VALUES
  ({{BASE_ACCOUNT_ID}}01);

INSERT INTO accounts (account_id, account_type_id, name, org_id, fields) VALUES
  ({{BASE_ACCOUNT_ID}}01, 'discord_incoming_webhook', 'Ergo Discord Webhook', '{{ORG_ID}}', '{"webhook_url": "{{DISCORD_WEBHOOK_URL}}"}');
