# LiteLLM Agent Platform

Self-hosted UI for creating and running agents on any agent runtime [Claude Managed Agents, Cursor Agents API, OpenCode Agents, DeepAgents]

<img width="2200" height="1245" alt="LiteLLM Agent Platform dashboard" src="https://github.com/user-attachments/assets/04333758-829c-4b19-bde3-23ade37bb9f1" />

## Why we built this

Agent runtimes don't come with a UI you can hand to your team:

- On **Bedrock AgentCore**, you have to build your own UI before anyone can create or run an agent.
- With **Anthropic managed agents**, the only UI is the Anthropic developer console, and you don't want every employee in there.

LiteLLM Agent Platform sits on top of any runtime. Pick a runtime, create an agent, give your team one UI.

It also manages:

- **Session management** - persistent agent sessions across runs
- **CRON schedules** - run agents on a schedule
- **Memory** - agents remember context across sessions

## Quick Start

Prerequisites: Docker Desktop, kind, kubectl, helm, and a running LiteLLM gateway.

```bash
git clone https://github.com/LiteLLM-Labs/litellm-agent-platform
cd litellm-agent-platform

bin/kind-up.sh     # provision local sandbox cluster
docker compose up  # start Postgres, web, and worker
```

Open http://localhost:3000 and create your first agent.

## Usage: Create an Agent

### 1. Make an agent in the UI

<img width="2200" height="1439" alt="Create agent screen" src="https://github.com/user-attachments/assets/d2083454-b7c1-4337-b2c2-4c4ba99991b6" />

### 2. Select tools and skills to connect to your agent

<img width="1870" height="1573" alt="Select tools and skills" src="https://github.com/user-attachments/assets/efd59a4e-dcc7-487a-923b-005ac44b44b0" />

### 3. Use your agent

Select your agent and the runtime you want to run it on.

<img width="2200" height="1570" alt="Run agent on a runtime" src="https://github.com/user-attachments/assets/be9cfd8c-4475-4309-bed0-4edcd7dd1de1" />

## Supported Agent Runtimes

- Claude Managed Agents
- Cursor Agents API
- OpenCode Agents
- DeepAgents

## Contributing

PRs welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).
