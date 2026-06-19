#!/usr/bin/env bash
# verify-runtimes.sh — fail loudly if any runtime can't actually reach an LLM.
#
# This is the guard for the failure that bit us on 2026-06-19: runtimes were
# pointed at an LLM endpoint that rejected their configured model name, so every
# agent run silently died with "unknown provider". Registration + connection
# can look green while runs are dead, so this checks the *LLM call path*, not
# just /health and harness registration.
#
# Run after `docker compose up`:  scripts/verify-runtimes.sh
# Exits non-zero on the first failure.
set -u

LAP_URL="${LAP_URL:-http://localhost:4000}"
LAP_KEY="${LITELLM_MASTER_KEY:-sk-local}"
fail=0
ok()   { printf '  \033[32mOK\033[0m   %s\n' "$1"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$1"; fail=1; }

echo "== LAP =="
if curl -fsS -m5 "$LAP_URL/health" >/dev/null 2>&1; then ok "LAP /health"; else bad "LAP /health unreachable at $LAP_URL"; fi

# LAP must proxy chat completions (it is the model-routing single source of truth).
resp=$(curl -sS -m30 "$LAP_URL/v1/chat/completions" \
  -H "Authorization: Bearer $LAP_KEY" -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-6","messages":[{"role":"user","content":"ping"}],"max_tokens":5}' 2>&1)
if echo "$resp" | grep -q '"choices"'; then ok "LAP proxies claude-sonnet-4-6 -> upstream"; else bad "LAP chat/completions: $resp"; fi

# Each LAP-routed runtime: confirm the OpenAI-compatible call it will make returns a completion.
# (deepagents + pydantic-deepagents route their LLM calls through LAP.)
for c in lap-deepagents-1 lap-pydantic-deepagents-1; do
  echo "== $c (LLM via LAP) =="
  if ! docker ps --format '{{.Names}}' | grep -qx "$c"; then echo "  (skip: not running)"; continue; fi
  out=$(docker exec -i "$c" python3 - <<'PY' 2>&1
import os, json, urllib.request, urllib.error
base=os.environ.get("OPENAI_BASE_URL") or os.environ.get("LITELLM_BASE_URL")
key=os.environ.get("OPENAI_API_KEY") or os.environ.get("LITELLM_API_KEY")
model=(os.environ.get("DEFAULT_MODEL") or "claude-sonnet-4-6").split("/")[-1].split(":")[-1]
req=urllib.request.Request(base.rstrip("/")+"/chat/completions",
    data=json.dumps({"model":model,"messages":[{"role":"user","content":"ping"}],"max_tokens":5}).encode(),
    headers={"Authorization":"Bearer "+key,"Content-Type":"application/json"})
try:
    r=json.loads(urllib.request.urlopen(req,timeout=30).read())
    print("OK" if r.get("choices") else "BAD:"+json.dumps(r)[:160])
except urllib.error.HTTPError as e:
    print("BAD: HTTP %s %s" % (e.code, e.read().decode()[:160]))
except Exception as e:
    print("BAD: %r" % e)
PY
)
  case "$out" in OK*) ok "$c default model answers via LAP" ;; *) bad "$c: $out" ;; esac
done

# opencode uses its own authenticated OpenCode Go subscription (NOT LAP/cliproxy).
echo "== lap-opencode-1 (OpenCode Go subscription) =="
if docker ps --format '{{.Names}}' | grep -qx lap-opencode-1; then
  models=$(docker exec lap-opencode-1 sh -c 'curl -sS -m8 localhost:8080/v1/models' 2>&1)
  n=$(echo "$models" | python3 -c "import sys,json;print(len(json.load(sys.stdin).get('data',[])))" 2>/dev/null || echo 0)
  if echo "$models" | grep -q 'opencode-go/' && [ "$n" -gt 0 ]; then
    ok "opencode lists $n opencode-go subscription models"
  else
    bad "opencode model list not from opencode-go subscription: $(echo "$models" | head -c 160)"
  fi
else echo "  (skip: not running)"; fi

# All registered runtimes report connected.
echo "== harness registration =="
curl -sS -m5 -H "Authorization: Bearer $LAP_KEY" "$LAP_URL/api/runtime-harnesses" 2>/dev/null \
  | python3 -c "
import sys,json
for h in json.load(sys.stdin).get('harnesses',[]):
    if h['alias'].startswith('local-'):
        print(('  \033[32mOK\033[0m   ' if h['connected'] else '  \033[31mFAIL\033[0m ')+h['alias']+' connected='+str(h['connected']))
" || bad "could not read harnesses"

echo
if [ "$fail" -eq 0 ]; then echo "All runtime LLM paths verified."; else echo "VERIFY FAILED — see FAIL lines above."; fi
exit $fail
