#!/bin/sh
set -e

# Ensure OpenAI environment variables are set for sandbox processes.
# OPENAI_BASE_URL MUST keep the /v1 suffix — the OpenAI client appends
# /chat/completions to it, and LAP only serves /v1/chat/completions (a bare
# /chat/completions returns 405). Normalize to exactly one trailing /v1.
export OPENAI_BASE_URL="${LITELLM_BASE_URL%/v1}/v1"
export OPENAI_API_KEY="${LITELLM_API_KEY}"
export DOCKER_SANDBOX_MODELS_URL="${DOCKER_SANDBOX_MODELS_URL:-${LITELLM_BASE_URL%/v1}/v1/models}"

# Also ensure they're in the environment for uvicorn
exec uvicorn src.server:app --host 0.0.0.0 --port "${PORT:-8080}"
