#!/usr/bin/env bash
#
# Test session memory persistence in Pydantic Deep Agents
#
# This test verifies that follow-up questions in the same session
# maintain conversation context/memory.

set -euo pipefail

BASE="${BASE:-http://localhost:8080}"
MODEL="${MODEL:-test}"  # Use pydantic's test model for local testing without API keys

HDR=(
  -H "content-type: application/json"
  -H "x-api-key: ${RUNTIME_API_KEY:-smoke}"
  -H "anthropic-version: 2023-06-01"
  -H "anthropic-beta: managed-agents-2026-04-01"
)

json_field() {
  local field="$1"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "import sys,json
try:
    d=json.load(sys.stdin)
except Exception:
    sys.exit(0)
v=d.get('$field','')
print(v if v is not None else '')" 2>/dev/null
  elif command -v node >/dev/null 2>&1; then
    node -e "let s='';process.stdin.on('data',d=>s+=d).on('end',()=>{try{const o=JSON.parse(s);process.stdout.write(String(o['$field']??''))}catch(e){}})" 2>/dev/null
  elif command -v jq >/dev/null 2>&1; then
    jq -r --arg f "$field" '.[$f] // empty' 2>/dev/null
  else
    echo "ERROR: need python3, node, or jq to parse JSON" >&2
    return 1
  fi
}

echo "=== Pydantic Deep Agents Session Memory Test ==="
echo

# Step 1: Create an agent
echo "1. Creating agent..."
agent_json=$(curl -s "${HDR[@]}" -X POST "$BASE/v1/agents" -d '{
  "name": "Memory Test Agent",
  "model": "'$MODEL'",
  "system": "You are a helpful assistant that remembers the conversation context. If the user mentions a name or topic from earlier, reference it."
}')
echo "Agent response: $agent_json"
aid=$(printf '%s' "$agent_json" | json_field id)
if [ -z "${aid:-}" ]; then
  echo "FAIL: no agent id returned" >&2
  exit 1
fi
echo "Agent ID: $aid"
echo

# Step 2: Create a session
echo "2. Creating session..."
session_json=$(curl -s "${HDR[@]}" -X POST "$BASE/v1/sessions" -d '{
  "agent": "'$aid'",
  "title": "memory-test"
}')
echo "Session response: $session_json"
sid=$(printf '%s' "$session_json" | json_field id)
if [ -z "${sid:-}" ]; then
  echo "FAIL: no session id returned" >&2
  exit 1
fi
echo "Session ID: $sid"
echo

# Step 3: Send first message and wait for completion
echo "3. Sending first message..."
first_response=$(curl -s "${HDR[@]}" -X POST "$BASE/v1/sessions/$sid/events" -d '{
  "events": [
    { "type": "user.message", "content": [ { "type": "text", "text": "My name is Alice. Remember this." } ] }
  ]
}')
echo "First message response: $first_response"
echo

# Wait for first message to complete
echo "4. Waiting for first message to complete..."
sleep 3

# Check session status
status_response=$(curl -s "${HDR[@]}" "$BASE/v1/sessions/$sid/events")
echo "Session events: $status_response"
echo

# Step 5: Send follow-up message that references the first
echo "5. Sending follow-up message..."
second_response=$(curl -s "${HDR[@]}" -X POST "$BASE/v1/sessions/$sid/events" -d '{
  "events": [
    { "type": "user.message", "content": [ { "type": "text", "text": "What is my name?" } ] }
  ]
}')
echo "Follow-up message response: $second_response"
echo

# Wait for follow-up to complete
echo "6. Waiting for follow-up to complete..."
sleep 3

# Step 7: Get all events and check if the agent remembered
echo "7. Checking if agent remembered the name..."
all_events=$(curl -s "${HDR[@]}" "$BASE/v1/sessions/$sid/events")
echo "All events:"
echo "$all_events" | python3 -m json.tool 2>/dev/null || echo "$all_events"
echo

# Check if the response contains "Alice"
if echo "$all_events" | grep -qi "Alice"; then
  echo "✓ SUCCESS: Agent remembered the name!"
  exit 0
else
  echo "✗ FAIL: Agent did not remember the name 'Alice'"
  exit 1
fi
