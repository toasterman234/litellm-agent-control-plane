# Developer Usage

End-to-end: create an agent, open a session, send a message, read the reply. Single-tenant Bearer auth — `MASTER_KEY` from `.env`.

```bash
export BASE=http://localhost:3000
export KEY=$MASTER_KEY
```

## 1. Create an agent

```bash
curl -s $BASE/api/v1/managed_agents/agents \
  -H "authorization: Bearer $KEY" \
  -H "content-type: application/json" \
  -d '{
    "name": "code-reviewer",
    "model": "anthropic/claude-opus-4-7",
    "prompt": "You review pull requests.",
    "repo_url": "https://github.com/BerriAI/litellm",
    "branch": "main"
  }'
```

Returns the new agent. Save `id` as `AGENT_ID`.

## 2. Create a session

Boots a Fargate task and the harness inside it. Cold start ~50–120s.

```bash
curl -s $BASE/api/v1/managed_agents/agents/$AGENT_ID/session \
  -H "authorization: Bearer $KEY" \
  -H "content-type: application/json" \
  -d '{
    "title": "review #1234",
    "initial_prompt": "Look at the diff in the latest PR."
  }'
```

Returns the new session. Save `id` as `SESSION_ID`. The agent's reply to `initial_prompt` is in `response`.

`initial_prompt` is optional — omit it to bring up the sandbox without sending anything.

## 3. Send a message

```bash
curl -s $BASE/api/v1/managed_agents/sessions/$SESSION_ID/message \
  -H "authorization: Bearer $KEY" \
  -H "content-type: application/json" \
  -d '{ "text": "Summarize the largest file changed." }'
```

Response body is the harness reply, synchronous. The message is appended to the session's history.

## 4. Read session state

```bash
curl -s $BASE/api/v1/managed_agents/sessions/$SESSION_ID \
  -H "authorization: Bearer $KEY"
```

Returns metadata plus the latest `response` blob.

## Cleanup

```bash
curl -X DELETE -s $BASE/api/v1/managed_agents/sessions/$SESSION_ID \
  -H "authorization: Bearer $KEY"
```

Stops the Fargate task and marks the session `dead`. Idempotent.
