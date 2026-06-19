# LAP Handoff — 2026-06-19

## Session Summary
Fixed "can't connect to the agents." Root cause: a **model-name mismatch**. The
runtimes were pointed straight at cliproxy and asked for model names cliproxy
rejects (`anthropic/claude-sonnet-4-5`, and the `anthropic/` prefix on any
name), so every deepagents / pydantic-deepagents run died at the LLM call with
`unknown provider for model …`. opencode worked; the other two were dead.

This traces back to the 2026-06-18 note below claiming **"LAP returns 405, it's
not an LLM proxy."** That was a **misdiagnosis** — LAP proxies fine
(`POST /v1/chat/completions` → 200). Its `config.yaml` is the model-name
normalization layer. Bypassing it removed that layer, so model names had to
exactly match cliproxy's ids — and the defaults didn't.

## What Was Done (2026-06-19)
- **Routed deepagents + pydantic-deepagents through LAP** (`config.yaml` is now
  the single source of model routing). Defaults set to `claude-sonnet-4-6`, a
  name LAP resolves to a working upstream.
  - deepagents uses the OpenAI path: `OPENAI_BASE_URL=http://lap:4000/v1`,
    model `openai/claude-sonnet-4-6`.
  - pydantic uses the same, bare model `claude-sonnet-4-6`.
- **opencode now uses its own authenticated OpenCode Go subscription** (mounted
  `auth.json`), NOT cliproxy. Removed its `LITELLM_*` vars so it can't fall
  back. `/v1/models` now lists the 13 opencode-go models (qwen/kimi/glm/…).
- **Added `langchain-openai`** to `templates/deepagents/requirements.txt`
  (the image shipped Anthropic-only).
- **Forced deepagents' ChatOpenAI to chat-completions** via
  `use_responses_api=False` in `templates/deepagents/src/server.py`
  (`build_model_for_deepagents`). langchain-openai ≥1.3 otherwise defaults to
  the Responses API at `/v1/responses`, which LAP does not serve → 404.
- **Fixed the `register-*` restart-loop** (`restart: "no"` — they are one-shot
  jobs that exit 0).
- **Rebuilt the opencode + deepagents images** (running containers were stale).
- **Added `scripts/verify-runtimes.sh`** — the guard. It checks the actual LLM
  call path for every runtime (not just /health + registration). Run it after
  every `docker compose up`. All checks currently green.

## Verification
- `scripts/verify-runtimes.sh` → all OK.
- Real deepagents agent (create agent → session → prompt) replies `AGENT ONLINE`
  through LAP.
- pydantic + opencode model paths confirmed.

## GOTCHAS (don't relearn)
- **LAP is a minimal Rust proxy.** It serves only `/v1/chat/completions` and
  `/v1/models`. It does NOT serve `/v1/responses` (OpenAI Responses API) or
  `/v1/messages` (Anthropic-native). Clients must use chat-completions.
- **cliproxy model names are exact, undated names fail.** It accepts
  `claude-sonnet-4-6` and `claude-sonnet-4-5-20250929`, and REJECTS the
  `anthropic/` prefix and the undated `claude-sonnet-4-5`. Route model names
  through LAP's `config.yaml` rather than hard-coding cliproxy ids in runtimes.
- **`docker compose up -d --force-recreate` reuses the existing image.** After
  editing anything under `templates/<runtime>/`, you must `docker compose build
  <runtime>` or the container runs stale code. (This caused opencode to keep
  showing cliproxy models despite `OPENCODE_PROVIDER_ID=opencode-go`.)
- **`register-*` containers exiting (0) is correct** — they register once and
  stop. Harness `connected: true` can be green while runs are dead; only
  `verify-runtimes.sh` proves the LLM path.
- **Latent trap in `config.yaml`:** the `anthropic/*` wildcard routes to the
  ZimaOS gateway (`192.168.1.121:4000`) with key `sk-local`, which that box
  rejects (401). Avoid `anthropic/*` model names; the explicit cliproxy-backed
  routes (`claude-sonnet-4-6`, `claude-opus-4-8`, `gpt-5.4`, …) are the ones
  that work.

## Architecture
- **cliproxy**: native Mac process at `127.0.0.1:8317`, key `sk-cliproxy` —
  OAuth LLM proxy. Serves OpenAI chat-completions AND Anthropic `/v1/messages`.
- **LAP**: Docker container at `localhost:4000`, key `sk-local` — agent control
  plane + minimal OpenAI-compatible model gateway (`config.yaml`).
- **Runtimes**: opencode (own OpenCode Go subscription), deepagents +
  pydantic-deepagents (LLM via LAP). All register as `claude_managed_agents`
  harnesses.

## Relevant Files
- `compose.yaml` — runtime env / routing
- `config.yaml` — LAP model routes (single source of model-name → upstream)
- `scripts/verify-runtimes.sh` — the guard
- `templates/deepagents/src/server.py` — `build_model_for_deepagents` (responses-API fix)
- `templates/deepagents/requirements.txt` — `langchain-openai`
- `.env` — master key, runtime API keys
