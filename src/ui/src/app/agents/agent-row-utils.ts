import type { Agent, AgentRuntimeId } from "@/lib/types";

export interface ImportedAgentSource {
  provider: string;
  credential_mode?: "shared" | "byo";
}

export function agentConfig(agent: Agent): Record<string, unknown> {
  return agent.config && typeof agent.config === "object" && !Array.isArray(agent.config)
    ? (agent.config as Record<string, unknown>)
    : {};
}

export function platformMcpIds(agent: Agent): string[] {
  const config = agentConfig(agent);
  const value = config.platform_mcp_ids ?? config.platformMcpIds;
  return Array.isArray(value) ? value.filter((id): id is string => typeof id === "string") : [];
}

export function subAgentIds(agent: Agent): string[] {
  const config = agentConfig(agent);
  const value = config.sub_agents ?? config.subAgents;
  if (!Array.isArray(value)) return [];
  return [
    ...new Set(
      value
        .map((entry) => {
          if (!entry || typeof entry !== "object") return "";
          const item = entry as Record<string, unknown>;
          const id = item.agent_id ?? item.agentId ?? item.id;
          return typeof id === "string" ? id.trim() : "";
        })
        .filter(Boolean),
    ),
  ];
}

export function importedSource(agent: Agent): ImportedAgentSource | null {
  const source = agentConfig(agent).source;
  if (!source || typeof source !== "object" || Array.isArray(source)) return null;
  const value = source as Record<string, unknown>;
  const provider = typeof value.provider === "string" ? value.provider.trim() : "";
  if (!provider) return null;
  const credentialMode = value.credential_mode;
  return {
    provider,
    credential_mode:
      credentialMode === "shared" || credentialMode === "byo" ? credentialMode : undefined,
  };
}

export function providerLabel(provider: string): string {
  return provider
    .split(/[_-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function runtimeFromAgent(agent: Agent): AgentRuntimeId {
  const runtime = agentConfig(agent).runtime;
  if (typeof runtime === "string" && runtime.trim()) return runtime;
  return typeof agent.harness === "string" && agent.harness.trim()
    ? agent.harness
    : "claude_managed_agents";
}
