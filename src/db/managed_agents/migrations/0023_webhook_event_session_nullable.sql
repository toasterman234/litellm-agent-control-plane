ALTER TABLE "LiteLLM_ManagedAgentWebhookEventsTable"
  ADD COLUMN IF NOT EXISTS claim_id TEXT;

UPDATE "LiteLLM_ManagedAgentWebhookEventsTable"
SET claim_id = 'legacy_' || md5(agent_id || ':' || event_id)
WHERE claim_id IS NULL;

ALTER TABLE "LiteLLM_ManagedAgentWebhookEventsTable"
  ALTER COLUMN claim_id SET NOT NULL;

ALTER TABLE "LiteLLM_ManagedAgentWebhookEventsTable"
  ALTER COLUMN session_id DROP NOT NULL;
