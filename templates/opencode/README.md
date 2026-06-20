# opencode Managed Agents server

Run this wrapper next to an [opencode](https://opencode.ai) install. It exposes
opencode through the Anthropic Managed Agents API so LiteLLM Agent Platform
(LAP) can use it as a `claude_managed_agents` runtime.

```text
LAP -> opencode-agent-server :8080 -> opencode serve :4096
```

## Quick Start

> **Important**
>
> This server starts `opencode serve` itself. Run it on the same machine where
> the `opencode` CLI is installed, or use Docker, which installs opencode inside
> the image for you.
>
> When using Docker, mount both `/data` and `/tmp/opencode-workspace` so the
> SQLite DB and generated opencode config survive restarts.

### Run with Docker

```bash
cd templates/opencode
docker build -t opencode-agent-server .
docker run --rm -p 8080:8080 \
  -v opencode-agent-data:/data \
  -v opencode-workspace:/tmp/opencode-workspace \
  -e LITELLM_BASE_URL=https://your-litellm-gateway/v1 \
  -e LITELLM_API_KEY=sk-... \
  opencode-agent-server
```

For a direct Anthropic key, use this environment line instead of the three
LiteLLM lines:

```bash
  -e ANTHROPIC_API_KEY=sk-ant-... \
```

Health check:

```bash
curl http://localhost:8080/health
# {"ok":true,"opencode":true}
```

If `opencode` is `false`, wait a few seconds and retry. The wrapper binds port
8080 first, then starts opencode in the background.

## Connect from LAP

In LAP, open **AI Gateway** > **Agent Runtimes** > **Add Runtime** and use:

| Field | Value |
|-------|-------|
| Alias | `opencode-local` |
| API Spec | `claude_managed_agents` |
| API Base | `http://localhost:8080` if LAP is on the same machine; `http://YOUR_SERVER_HOST:8080` or your HTTPS URL if LAP is elsewhere |
| API Key | Any non-empty placeholder, for example `fake-opencode-key` |

The server accepts Anthropic-style headers but does not validate the inbound API
key. LAP still needs a value so it can store the runtime credential.

After adding the runtime, select `opencode-local` in a LAP session and use a
model returned by this wrapper's `/v1/models` endpoint.

## Per-agent model selection

`model` is a **string** in the route contract. Create an agent with the model
you want and every turn in its sessions runs on that model:

```json
POST /v1/agents
{
  "name": "Agent",
  "model": "gpt-5.5",
  "system": "..."
}
```

The model string is stored on the agent and returned from `GET /v1/agents/:id`.

For LiteLLM-backed deployments, a bare name like `gpt-5.5` is normalized to
opencode's provider/model object `{ "providerID": "litellm", "modelID": "gpt-5.5" }`,
and the model is registered in the generated `opencode.json` before opencode
reboots. Pass `litellm/gpt-5.5` to set the provider/model split explicitly. The
target model must be routable by your configured LiteLLM gateway.

For direct OpenCode providers such as `opencode-go`, set `OPENCODE_PROVIDER_ID`
to the provider id. Bare model names then normalize to that provider, and
`GET /v1/models` is sourced from `opencode models <provider>`. For example, if
`OPENCODE_PROVIDER_ID=opencode-go`, a bare model like `qwen3.7-plus` becomes
`opencode-go/qwen3.7-plus`.

When using an authenticated OpenCode provider in Docker, mount the host
credential file into the container so the bundled `opencode` CLI can see the
same login state:

```bash
  -v ~/.local/share/opencode/auth.json:/root/.local/share/opencode/auth.json:ro
```

> The object form `{ "id": "gpt-5.5" }` is still accepted for backward
> compatibility, but the string form is the documented API.

Smoke-test a specific model end to end:

```bash
BASE=http://localhost:8080 MODEL=gpt-5.5 ./scripts/smoke.sh
```

## Reuse an existing OpenCode config

If you already have a working OpenCode setup, the wrapper can seed its runtime
workspace from that config before LAP writes provider and agent-specific state.

Set these environment variables on the container:

| Var | Purpose |
|-----|---------|
| `OPENCODE_BASE_CONFIG_PATH` | path to a base `opencode.json` to copy into the runtime workspace at boot |
| `OPENCODE_MEMORY_PLUGIN_CONFIG_PATH` | optional path to a `memory-plugin.json` file copied to `.opencode/memory-plugin.json` |

This preserves top-level settings such as `experimental`, `plugin`, and any
static MCP entries in the base config. LAP still rebuilds agent-attached
`mcp_servers` on each change, so agent MCPs stay authoritative.

For plugin packages or local plugin directories, mount your OpenCode home into
the container as `/root/.config/opencode` so `opencode serve` can resolve the
same plugin dependencies it uses outside LAP.

## Environment variables

| Var | Default | Purpose |
|-----|---------|---------|
| `PORT` | `8080` | listen port |
| `OPENCODE_PORT` | `4096` | internal `opencode serve` port |
| `WORKDIR` | `/tmp/opencode-workspace` | per-agent config directory |
| `DB_PATH` | `/data/agents.db` | SQLite agent store |
| `ANTHROPIC_API_KEY` | — | native Anthropic key (alternative to LiteLLM) |
| `LITELLM_BASE_URL` | — | LiteLLM gateway base URL (include `/v1`) |
| `LITELLM_API_KEY` | — | LiteLLM gateway key |
| `LITELLM_MODELS` | — | optional comma-separated models to pre-register in opencode |
| `OPENCODE_PROVIDER_ID` | — | optional authenticated built-in provider id such as `opencode-go`; takes precedence over LiteLLM for model listing and bare model normalization |
| `OPENCODE_AUTH_JSON_PATH` | — | host path to the OpenCode `auth.json` file when using Docker Compose with an authenticated provider |
| `OPENCODE_BASE_CONFIG_PATH` | — | optional base `opencode.json` to seed before LAP merges provider/agent config |
| `OPENCODE_MEMORY_PLUGIN_CONFIG_PATH` | — | optional `memory-plugin.json` copied into `.opencode/` at boot |
| `OPENSANDBOX_API_URL` | — | OpenSandbox controller URL (enables sandboxed execution) |
| `OPENSANDBOX_API_KEY` | — | OpenSandbox API key |
| `OPENSANDBOX_IMAGE` | — | sandbox execd image |

## Deploy on EKS with OpenSandbox

For a production deployment of opencode-anthropic-server + OpenSandbox on EKS,
see [`docs/eks-deployment.md`](docs/eks-deployment.md).
