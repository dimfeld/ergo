INSERT INTO notify_endpoints
  (notify_endpoint_id, org_id, service, destination)
  VALUES
  (objectid_to_uuid('{{DISCORD_NOTIFY_ENDPOINT_ID}}'), objectid_to_uuid('{{ORG_ID}}'), 'discord_incoming_webhook', '{{DISCORD_WEBHOOK_URL}}');

INSERT INTO notify_listeners
  (notify_listener_id, org_id, notify_endpoint_id, object_id, event)
  VALUES
  (objectid_to_uuid('{{DISCORD_NOTIFY_LISTENER_INPUT_PROCESSED_ID}}'), objectid_to_uuid('{{ORG_ID}}'), objectid_to_uuid('{{DISCORD_NOTIFY_ENDPOINT_ID}}'), 1, 'input_processed'),
  (objectid_to_uuid('{{DISCORD_NOTIFY_LISTENER_ACTION_STARTED_ID}}'), objectid_to_uuid('{{ORG_ID}}'), objectid_to_uuid('{{DISCORD_NOTIFY_ENDPOINT_ID}}'), 1, 'action_started'),
  (objectid_to_uuid('{{DISCORD_NOTIFY_LISTENER_ACTION_SUCCESS_ID}}'), objectid_to_uuid('{{ORG_ID}}'), objectid_to_uuid('{{DISCORD_NOTIFY_ENDPOINT_ID}}'), 1, 'action_success'),
  (objectid_to_uuid('{{DISCORD_NOTIFY_LISTENER_ACTION_ERROR_ID}}'), objectid_to_uuid('{{ORG_ID}}'), objectid_to_uuid('{{DISCORD_NOTIFY_ENDPOINT_ID}}'), 1, 'action_error');

