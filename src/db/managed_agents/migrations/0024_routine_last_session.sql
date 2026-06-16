ALTER TABLE "LiteLLM_ManagedAgentRoutinesTable"
  ADD COLUMN IF NOT EXISTS last_session_id TEXT REFERENCES "LiteLLM_ManagedAgentSessionsTable" (id) ON DELETE SET NULL;
