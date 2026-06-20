# Session Memory Fix for Pydantic Deep Agents

## Problem

When using pydantic-deepagents in LAP, follow-up questions in the same chat session were losing memory. The agent would not remember previous conversation context, making each message feel like a new, isolated interaction.

## Root Causes

After investigation, **three critical issues** were identified:

### 1. Wrong History File Path (Line 837)
**Original:**
```python
history_messages_path=f".pydantic-deep/{row['id']}/messages.json",
```

**Issue:** `row['id']` is the **agent ID**, not the session ID. This caused all sessions using the same agent to share one history file, with each new session overwriting the previous one's messages.

**Fix:**
```python
history_messages_path=str(session_workdir(session_id) / ".pydantic-deep" / "messages.json"),
```

This uses the **session ID** to create a unique path per session within the session's workspace directory.

---

### 2. Missing Session ID Parameter
**Original:**
```python
def build_agent(row: sqlite3.Row, backend: Any) -> tuple[Any, list[Any]]:
    # ...
    history_messages_path=f".pydantic-deep/{row['id']}/messages.json",

# Called from:
agent, mcp_toolsets = build_agent(agent_row, backend)
```

**Issue:** The `build_agent` function didn't receive the `session_id`, so it couldn't create a session-specific history path.

**Fix:**
```python
def build_agent(row: sqlite3.Row, backend: Any, session_id: str) -> tuple[Any, list[Any]]:
    # ...
    history_messages_path=str(session_workdir(session_id) / ".pydantic-deep" / "messages.json"),

# Called from:
agent, mcp_toolsets = build_agent(agent_row, backend, session_id)
```

---

### 3. Incoming Messages Not Saved to Database
**Original:** The `send_events` function extracted the prompt text but didn't save the incoming `user.message` events to the database.

**Issue:** When `load_conversation_history()` was called for follow-up messages, it couldn't find previous messages because they were never persisted.

**Fix:** Added code to save incoming user.message events:
```python
# Save incoming user.message events to database for history
for event in input.events:
    if event.get("type") == "user.message":
        append_event(session_id, "user.message", event)
```

---

### 4. No History Loading for Follow-up Prompts
**Original:** Each message created a new agent instance with only the current prompt.

**Issue:** Even with the correct history path, each new agent instance started fresh without loading previous conversation context.

**Fix:** Added `load_conversation_history()` function that:
1. Loads all previous user.message and agent.message events from the session
2. Formats them with "User:" and "Assistant:" prefixes
3. Prepends the formatted history to the current prompt

```python
def load_conversation_history(session_id: str) -> str:
    """Load previous user and assistant messages from session events."""
    events = list_events(session_id)
    messages: list[str] = []
    for event in events:
        # ... extract messages from events
        
    return "\n".join(messages) if messages else ""

# In run_agent_once:
history = load_conversation_history(session_id)
if history:
    prompt = f"{history}\n\nUser: {prompt}"
```

## Changes Made

### File: `templates/pydantic-deepagents/src/server.py`

| Line | Change |
|------|--------|
| 801 | Updated `build_agent()` signature to accept `session_id` parameter |
| 837 | Changed `history_messages_path` to use session-specific path |
| 851 | Updated `build_agent()` call to pass `session_id` |
| 859-862 | Added history loading and prepending to prompt in `run_agent_once()` |
| 527-546 | Added `load_conversation_history()` function |
| 1131-1133 | Added code to save incoming user.message events to database |

## How It Works Now

1. **Session Creation**: When a session is created, it gets a unique workspace directory
2. **Message History Path**: Each session's agent uses `PYDANTIC_DEEP_WORKDIR_ROOT/session_id/.pydantic-deep/messages.json`
3. **Incoming Messages**: User messages are saved to the SQLite database immediately upon receipt
4. **History Loading**: Before processing each message, the server loads all previous messages from the session and prepends them to the prompt
5. **Agent Context**: The agent receives the full conversation history as part of its prompt, maintaining context across messages

## Testing

A test script has been added: `scripts/test_session_memory.sh`

To test locally:

```bash
cd templates/pydantic-deepagents

# Start the server with test configuration
RUNTIME_API_KEY=test-key \
  DB_PATH=/tmp/pydantic-deep-test.db \
  PYDANTIC_DEEP_WORKDIR_ROOT=/tmp/pydantic-deep-test-workspaces \
  PYDANTIC_DEEP_MEMORY=false \
  PYDANTIC_DEEP_TODO=false \
  PYDANTIC_DEEP_FILESYSTEM=false \
  PYDANTIC_DEEP_SUBAGENTS=false \
  PYDANTIC_DEEP_SKILLS=false \
  .venv/bin/uvicorn src.server:app --host 0.0.0.0 --port 8080

# In another terminal, run the test
BASE=http://localhost:8080 MODEL=test RUNTIME_API_KEY=test-key \
  bash scripts/test_session_memory.sh
```

## Additional Considerations

### Memory Feature vs History Messages

Pydantic Deep Agents has a separate **memory feature** (enabled by `include_memory=True`) that uses file-based persistent memory. This was causing issues because:
- It tried to access `/ .deep/memory/main/MEMORY.md` (absolute path)
- This path was outside the allowed sandbox directory
- The memory feature uses a different path than `history_messages_path`

**Current approach**: We disabled the memory feature (`PYDANTIC_DEEP_MEMORY=false`) and instead manually manage conversation history by loading previous messages and including them in the prompt. This is more reliable and doesn't have file permission issues.

### Production Deployment

For production use with real models:
- You can enable `PYDANTIC_DEEP_MEMORY=true` if you configure proper file permissions
- Or keep it disabled and rely on the manual history loading (which works for all models)
- The manual history approach (loading and prepending messages) works regardless of the memory feature setting

## Performance Impact

The history loading adds a small overhead:
- One database query per message to load previous events
- String concatenation to build the prompt with history
- Slightly longer prompts for the LLM

This is a reasonable trade-off for maintaining conversation context.

## Backward Compatibility

These changes are backward compatible:
- Existing sessions will work as before
- New sessions will benefit from the memory fix
- The changes only affect the pydantic-deepagents template, not other runtimes
