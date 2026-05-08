# LiteLLM Agent Platform 🚄

A web UI for managing **agents** and their **sandboxed coding sessions** on a [LiteLLM](https://github.com/BerriAI/litellm) proxy. Each agent is bound to a sandbox template (a harness — opencode, claude-code, etc. — paired with a repo). Spawning a session boots a fresh Fargate task running that harness against that repo, and the proxy handles the lifecycle.

This UI is the front-end half of [BerriAI/litellm#27427](https://github.com/BerriAI/litellm/pull/27427). Point it at a LiteLLM proxy with `general_settings.managed_agents.enabled: true`.

![Agents list](./docs/screenshots/agents.png)

![Agent detail with the 'Call this agent' card](./docs/screenshots/agent-detail.png)

## Deploy

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template?template=https%3A%2F%2Fgithub.com%2FBerriAI%2Flitellm-agent-platform)
[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/BerriAI/litellm-agent-platform)

Set two **server-side** env vars on the host. They must NOT have a
`NEXT_PUBLIC_` prefix — that would bundle them into the browser JS.

| Var | Example |
| --- | --- |
| `LITELLM_BASE_URL` | `https://your-proxy.example.com` |
| `LITELLM_API_KEY` | `sk-...` (master key or virtual key) |

The browser never talks to the LiteLLM proxy directly. It calls
`/api/proxy/[...path]` on this app, a Next.js Route Handler that reads
`LITELLM_API_KEY` server-side and attaches the `Authorization` header on the
outbound request. Inspecting the page bundle / network tab will not reveal
the key.

## Run locally

```bash
npm install
cp .env.local.example .env.local   # set LITELLM_BASE_URL + LITELLM_API_KEY
npm run dev                         # http://localhost:3000
```

## Endpoints used

The browser hits `/api/proxy/<path>`. The route handler forwards each request
to `${LITELLM_BASE_URL}/<path>` with the API key attached:

```
GET    /v1/managed_agents/dockerfiles
GET    /v1/managed_agents/sandbox-templates
POST   /v1/managed_agents/agents
GET    /v1/managed_agents/agents
GET    /v1/managed_agents/agents/{id}
PATCH  /v1/managed_agents/agents/{id}                # name + pfp_url + mcp_servers
POST   /v1/managed_agents/agents/{id}/session        # ~50–90s spawn
GET    /v1/managed_agents/sessions
GET    /v1/managed_agents/sessions/{id}
POST   /v1/managed_agents/sessions/{id}/message
GET    /v1/managed_agents/sessions/{id}/events       # SSE
DELETE /v1/managed_agents/sessions/{id}
GET    /v1/mcp/server                                 # for the MCP picker
GET    /v1/models                                     # for the model picker
```

## Stack

- Next.js 16 App Router + React 19
- shadcn/ui + Tailwind v4
- Server-side proxy route — API key never leaves the server
