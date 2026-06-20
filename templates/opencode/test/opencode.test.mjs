import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { ensureProviderModel, seedBaseConfig, writeMcpConfig, writeProviderConfig } from "../src/opencode.mjs";

test("ensureProviderModel adds newly requested LiteLLM models", async () => {
  const cwd = await mkdtemp(path.join(tmpdir(), "opencode-provider-"));
  try {
    await writeProviderConfig(cwd, {
      id: "litellm",
      baseURL: "https://gateway.example/v1",
      apiKey: "test-key",
      models: ["claude-sonnet-4-5"],
    });
    await ensureProviderModel(cwd, {
      providerID: "litellm",
      modelID: "claude-sonnet-4-6",
    });

    const config = JSON.parse(await readFile(path.join(cwd, "opencode.json"), "utf8"));
    assert.deepEqual(Object.keys(config.provider.litellm.models).sort(), [
      "claude-sonnet-4-5",
      "claude-sonnet-4-6",
    ]);
  } finally {
    await rm(cwd, { recursive: true, force: true });
  }
});

test("ensureProviderModel registers gpt-5.5 while preserving existing models", async () => {
  const cwd = await mkdtemp(path.join(tmpdir(), "opencode-provider-"));
  try {
    await writeProviderConfig(cwd, {
      id: "litellm",
      baseURL: "https://gateway.example/v1",
      apiKey: "test-key",
      models: ["claude-sonnet-4-6"],
    });
    await ensureProviderModel(cwd, { providerID: "litellm", modelID: "gpt-5.5" });

    const config = JSON.parse(await readFile(path.join(cwd, "opencode.json"), "utf8"));
    assert.deepEqual(Object.keys(config.provider.litellm.models).sort(), [
      "claude-sonnet-4-6",
      "gpt-5.5",
    ]);
  } finally {
    await rm(cwd, { recursive: true, force: true });
  }
});

test("seedBaseConfig copies a base config and memory plugin config into the workspace", async () => {
  const cwd = await mkdtemp(path.join(tmpdir(), "opencode-seed-"));
  const fixtureDir = await mkdtemp(path.join(tmpdir(), "opencode-fixture-"));
  try {
    const baseConfigPath = path.join(fixtureDir, "opencode.json");
    const memoryConfigPath = path.join(fixtureDir, "memory-plugin.json");
    await writeFile(
      baseConfigPath,
      JSON.stringify({
        experimental: { openTelemetry: true },
        mcp: {
          github: { type: "local", command: ["github-mcp"], enabled: true },
        },
        plugin: ["opencode-plugin-process-validator"],
      }),
      "utf8"
    );
    await writeFile(memoryConfigPath, JSON.stringify({ memoryService: { endpoint: "http://memory.local" } }), "utf8");

    await seedBaseConfig(cwd, {
      baseConfigPath,
      memoryPluginConfigPath: memoryConfigPath,
    });

    const config = JSON.parse(await readFile(path.join(cwd, "opencode.json"), "utf8"));
    const memoryConfig = JSON.parse(await readFile(path.join(cwd, ".opencode", "memory-plugin.json"), "utf8"));
    assert.equal(config.experimental.openTelemetry, true);
    assert.deepEqual(config.plugin, ["opencode-plugin-process-validator"]);
    assert.deepEqual(config.mcp.github.command, ["github-mcp"]);
    assert.equal(memoryConfig.memoryService.endpoint, "http://memory.local");
  } finally {
    await rm(cwd, { recursive: true, force: true });
    await rm(fixtureDir, { recursive: true, force: true });
  }
});

test("writeMcpConfig preserves static base MCP entries while rebuilding agent MCPs", async () => {
  const cwd = await mkdtemp(path.join(tmpdir(), "opencode-static-mcp-"));
  const fixtureDir = await mkdtemp(path.join(tmpdir(), "opencode-static-mcp-fixture-"));
  try {
    const baseConfigPath = path.join(fixtureDir, "opencode.json");
    await writeFile(
      baseConfigPath,
      JSON.stringify({
        mcp: {
          github: { type: "local", command: ["github-mcp"], enabled: true },
        },
      }),
      "utf8"
    );
    await seedBaseConfig(cwd, { baseConfigPath });

    await writeMcpConfig(cwd, [
      {
        mcp_servers: [
          { name: "repo-docs", command: "python3", args: ["server.py"] },
        ],
      },
    ]);

    const config = JSON.parse(await readFile(path.join(cwd, "opencode.json"), "utf8"));
    assert.deepEqual(Object.keys(config.mcp).sort(), ["github", "repo-docs"]);
    assert.deepEqual(config.mcp.github.command, ["github-mcp"]);
    assert.deepEqual(config.mcp["repo-docs"].command, ["python3", "server.py"]);
  } finally {
    await rm(cwd, { recursive: true, force: true });
    await rm(fixtureDir, { recursive: true, force: true });
  }
});
