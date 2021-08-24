INSERT INTO notify_endpoints
  (notify_endpoint_id, org_id, service, destination)
  VALUES
  ({{BASE_NOTIFY_ENDPOINT_ID}}01, '{{ORG_UUID}}', 'discord_incoming_webhook', '{{DISCORD_WEBHOOK_URL}}');

INSERT INTO notify_listeners
  (notify_listener_id, org_id, notify_endpoint_id, object_id, event)
  VALUES
  ({{BASE_NOTIFY_LISTENER_ID}}001, '{{ORG_UUID}}', {{BASE_NOTIFY_ENDPOINT_ID}}01, 1, 'input_processed'),
  ({{BASE_NOTIFY_LISTENER_ID}}002, '{{ORG_UUID}}', {{BASE_NOTIFY_ENDPOINT_ID}}01, 1, 'action_started'),
  ({{BASE_NOTIFY_LISTENER_ID}}003, '{{ORG_UUID}}', {{BASE_NOTIFY_ENDPOINT_ID}}01, 1, 'action_success'),
  ({{BASE_NOTIFY_LISTENER_ID}}004, '{{ORG_UUID}}', {{BASE_NOTIFY_ENDPOINT_ID}}01, 1, 'action_error');
  
