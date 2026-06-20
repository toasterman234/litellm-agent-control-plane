// Route-level tests for the Anthropic Managed Agents surface in app.mjs.
//
// The opencode child and provider-config writers are faked via injected deps,
// so these tests exercise the real Express routing + the real (in-memory)
// SQLite store without booting `opencode serve`. The focus is the per-agent
// model contract: a string model on POST /v1/agents must survive storage,
// read-back, and reach opencode's prompt_async as a normalized provider/model.
import assert from "node:assert/strict";
import test from "node:test";

import { createApp } from "../src/app.mjs";
import { createStore } from "../src/store.mjs";

// A minimal fetch-Response stand-in for the opencode fake.
function fakeRes(body) {
  return {
    ok: true,
    status: 200,
    json: async () => body,
    text: async () => JSON.stringify(body),
  };
}

function fakeSse(events = [], { ok = true, status = 200 } = {}) {
  if (!ok) {
    return {
      ok: false,
      status,
      body: null,
      json: async () => ({}),
      text: async () => "",
    };
  }

  const encoder = new TextEncoder();
  return {
    ok: true,
    status,
    body: new ReadableStream({
      start(controller) {
        for (const ev of events) {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify(ev)}\n\n`));
        }
        controller.close();
      },
    }),
    json: async () => ({}),
    text: async () => "",
  };
}

// Build the app with fake opencode collaborators and capture their calls.
function buildHarness({ eventStreams = [[]], eventResponse = null, messageResponses = [] } = {}) {
  const store = createStore(":memory:");
  const calls = { ocFetch: [], provisionAgent: [], ensureProviderModel: [], reboots: 0 };
  let eventStreamIndex = 0;
  let messageResponseIndex = 0;

  const app = createApp({
    store,
    workdir: "/tmp/test-workspace",
    defaultModelProviderID: "litellm",
    registeredModelProviderID: "litellm",
    listModels: async () => ({
      object: "list",
      data: [
        { id: "gpt-5.5", object: "model", created: 0, owned_by: "litellm" },
      ],
    }),
    ensureProviderModel: async (cwd, model) => {
      calls.ensureProviderModel.push({ cwd, model });
    },
    provisionAgent: async (cwd, agent) => {
      calls.provisionAgent.push({ cwd, agent });
    },
    writeMcpConfig: async () => {},
    rebootOpencode: async () => {
      calls.reboots += 1;
    },
    ocBase: async () => "http://fake-opencode",
    ocFetch: async (baseUrl, path, init) => {
      calls.ocFetch.push({ path, init });
      if (path === "/session") return fakeRes({ id: "ses_test" });
      if (path.endsWith("/prompt_async")) return fakeRes({ ok: true });
      if (path === "/event") {
        if (eventResponse) return eventResponse();
        return fakeSse(eventStreams[eventStreamIndex++] || []);
      }
      if (path.endsWith("/message")) {
        return fakeRes(messageResponses[messageResponseIndex++] || []);
      }
      return fakeRes({});
    },
    checkOpencode: async () => true,
  });

  return { app, store, calls };
}

// Start the app on an ephemeral port and return a base URL + closer.
function listen(app) {
  return new Promise((resolve) => {
    const server = app.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      resolve({ base: `http://127.0.0.1:${port}`, close: () => server.close() });
    });
  });
}

async function req(base, method, path, body) {
  const res = await fetch(base + path, {
    method,
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  let json;
  try {
    json = JSON.parse(text);
  } catch {
    json = undefined;
  }
  return { status: res.status, json, text };
}

async function waitFor(check, { timeoutMs = 1500, intervalMs = 25 } = {}) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const result = await check();
    if (result) return result;
    if (Date.now() >= deadline) return null;
    await new Promise((r) => setTimeout(r, intervalMs));
  }
}

test("GET /v1/models returns the configured OpenAI-shaped model list", async (t) => {
  const { app } = buildHarness();
  const { base, close } = await listen(app);
  t.after(() => close());

  const models = await req(base, "GET", "/v1/models");
  assert.equal(models.status, 200);
  assert.deepEqual(models.json, {
    object: "list",
    data: [
      { id: "gpt-5.5", object: "model", created: 0, owned_by: "litellm" },
    ],
  });
});

test("per-agent gpt-5.5 model flows from create through prompt_async", async (t) => {
  const { app, calls } = buildHarness();
  const { base, close } = await listen(app);
  t.after(() => close());

  // 1. POST /v1/agents with a string model stores the model string.
  const created = await req(base, "POST", "/v1/agents", {
    name: "GPT Agent",
    model: "gpt-5.5",
    system: "You are terse.",
  });
  assert.equal(created.status, 200);
  assert.equal(created.json.model.id, "gpt-5.5");
  const agentId = created.json.id;
  assert.ok(agentId, "agent id returned");

  // The LiteLLM-routed bare model is registered before provisioning, and the
  // agent config is provisioned with the normalized provider/model string.
  assert.deepEqual(calls.ensureProviderModel.at(-1).model, {
    providerID: "litellm",
    modelID: "gpt-5.5",
  });
  assert.equal(calls.provisionAgent.at(-1).agent.model, "litellm/gpt-5.5");

  // 2. GET /v1/agents/:id returns the created agent with that model.
  const fetched = await req(base, "GET", `/v1/agents/${agentId}`);
  assert.equal(fetched.status, 200);
  assert.equal(fetched.json.model.id, "gpt-5.5");

  // 3. POST /v1/sessions binds the session to the agent.
  const session = await req(base, "POST", "/v1/sessions", { agent: agentId });
  assert.equal(session.status, 200);
  assert.equal(session.json.id, "ses_test");
  assert.equal(session.json.agent, agentId);

  // 4. POST /v1/sessions/:id/events calls opencode prompt_async with the
  //    normalized model for that agent.
  const events = await req(base, "POST", "/v1/sessions/ses_test/events", {
    events: [{ type: "user.message", content: "Say hello." }],
  });
  assert.equal(events.status, 202);

  const prompt = calls.ocFetch.find((c) => c.path.endsWith("/prompt_async"));
  assert.ok(prompt, "prompt_async was called");
  const sent = JSON.parse(prompt.init.body);
  assert.equal(sent.agent, agentId);
  assert.deepEqual(sent.model, { providerID: "litellm", modelID: "gpt-5.5" });
  assert.deepEqual(sent.parts, [{ type: "text", text: "Say hello." }]);
  assert.equal(calls.ocFetch.some((c) => c.path === "/event"), true);
  assert.equal(calls.ocFetch.some((c) => c.path === "/session/ses_test/prompt_async"), true);
});

test("litellm/gpt-5.5 keeps its explicit provider split end to end", async (t) => {
  const { app, calls } = buildHarness();
  const { base, close } = await listen(app);
  t.after(() => close());

  const created = await req(base, "POST", "/v1/agents", {
    name: "Explicit",
    model: "litellm/gpt-5.5",
    system: "",
  });
  assert.equal(created.json.model.id, "litellm/gpt-5.5");
  assert.equal(calls.provisionAgent.at(-1).agent.model, "litellm/gpt-5.5");

  await req(base, "POST", "/v1/sessions", { agent: created.json.id });
  await req(base, "POST", "/v1/sessions/ses_test/events", {
    events: [{ type: "user.message", content: "hi" }],
  });

  const prompt = calls.ocFetch.find((c) => c.path.endsWith("/prompt_async"));
  const sent = JSON.parse(prompt.init.body);
  assert.deepEqual(sent.model, { providerID: "litellm", modelID: "gpt-5.5" });
});

test("POST /v1/sessions rejects an unknown agent", async (t) => {
  const { app } = buildHarness();
  const { base, close } = await listen(app);
  t.after(() => close());

  const session = await req(base, "POST", "/v1/sessions", { agent: "agt_missing" });
  assert.equal(session.status, 400);
  assert.match(session.json.error, /unknown agent/);
});

test("GET /v1/sessions/:id/events returns flat replay events with type", async (t) => {
  const rawEvents = [
    {
      type: "message.part.delta",
      properties: {
        sessionID: "ses_test",
        messageID: "msg_1",
        partID: "part_1",
        delta: { text: "hello" },
      },
    },
    {
      type: "session.status",
      properties: {
        sessionID: "ses_test",
        status: { type: "idle" },
      },
    },
  ];
  const { app } = buildHarness({ eventStreams: [rawEvents] });
  const { base, close } = await listen(app);
  t.after(() => close());

  const created = await req(base, "POST", "/v1/agents", {
    name: "Replay",
    model: "gpt-5.5",
  });
  await req(base, "POST", "/v1/sessions", { agent: created.json.id });
  const sent = await req(base, "POST", "/v1/sessions/ses_test/events", {
    events: [{ type: "user.message", content: "hi" }],
  });
  assert.equal(sent.status, 202);

  const history = await req(base, "GET", "/v1/sessions/ses_test/events");
  assert.equal(history.status, 200);
  assert.deepEqual(history.json.data.map((e) => e.type), [
    "user.message",
    "agent.message",
    "session.status_idle",
  ]);
  assert.equal(history.json.data[1].content[0].text, "hello");
  assert.equal("event" in history.json.data[0], false);
  assert.equal("data" in history.json.data[0], false);
});

test("POST /v1/sessions/:id/events falls back to session message polling when /event is unavailable", async (t) => {
  const future = Date.now() + 60_000;
  const assistantMessage = {
    info: {
      id: "msg_assistant",
      sessionID: "ses_test",
      role: "assistant",
      time: { created: future, completed: future + 10 },
    },
    parts: [
      {
        id: "part_text",
        sessionID: "ses_test",
        messageID: "msg_assistant",
        type: "text",
        text: "Hello from polling fallback.",
      },
    ],
  };
  const { app, calls } = buildHarness({
    eventResponse: () => fakeSse([], { ok: false, status: 503 }),
    messageResponses: [[assistantMessage]],
  });
  const { base, close } = await listen(app);
  t.after(() => close());

  const created = await req(base, "POST", "/v1/agents", {
    name: "Capture failure",
    model: "gpt-5.5",
  });
  await req(base, "POST", "/v1/sessions", { agent: created.json.id });
  const sent = await req(base, "POST", "/v1/sessions/ses_test/events", {
    events: [{ type: "user.message", content: "hi" }],
  });

  assert.equal(sent.status, 202);
  assert.equal(calls.ocFetch.some((c) => c.path.endsWith("/prompt_async")), true);

  const history = await waitFor(async () => {
    const current = await req(base, "GET", "/v1/sessions/ses_test/events");
    return current.json.data.length >= 3 ? current : null;
  });
  assert.ok(history, "history should include fallback assistant output");
  assert.deepEqual(history.json.data.map((e) => e.type), [
    "user.message",
    "agent.message",
    "session.status_idle",
  ]);
  assert.equal(history.json.data[1].content[0].text, "Hello from polling fallback.");
});

test("background capture and live stream dedupe no-id events", async (t) => {
  const rawEvents = [
    {
      type: "message.part.delta",
      properties: {
        sessionID: "ses_test",
        messageID: "msg_1",
        partID: "part_1",
        delta: { text: "same" },
      },
    },
    {
      type: "session.status",
      properties: {
        status: { type: "idle" },
      },
    },
  ];
  const { app } = buildHarness({
    eventStreams: [rawEvents, rawEvents],
    messageResponses: [[
      {
        info: {
          id: "msg_1",
          sessionID: "ses_test",
          role: "assistant",
          time: { created: Date.now() + 60_000, completed: Date.now() + 60_010 },
        },
        parts: [
          {
            id: "part_1",
            sessionID: "ses_test",
            messageID: "msg_1",
            type: "text",
            text: "same",
          },
        ],
      },
    ]],
  });
  const { base, close } = await listen(app);
  t.after(() => close());

  const created = await req(base, "POST", "/v1/agents", {
    name: "Dedupe",
    model: "gpt-5.5",
  });
  await req(base, "POST", "/v1/sessions", { agent: created.json.id });
  const sent = await req(base, "POST", "/v1/sessions/ses_test/events", {
    events: [{ type: "user.message", content: "hi" }],
  });
  assert.equal(sent.status, 202);

  const stream = await fetch(base + "/v1/sessions/ses_test/events/stream");
  assert.equal(stream.status, 200);
  const streamText = await stream.text();
  assert.match(streamText, /event: agent\.message/);
  assert.match(streamText, /event: session\.status_idle/);

  const history = await waitFor(async () => {
    const current = await req(base, "GET", "/v1/sessions/ses_test/events");
    return current.json.data.some((e) => e.type === "session.status_idle") ? current : null;
  });
  assert.ok(history, "history should include terminal idle");
  const types = history.json.data.map((e) => e.type);
  assert.equal(types[0], "user.message");
  assert.equal(types.includes("agent.message"), true);
  assert.equal(types.includes("session.status_idle"), true);
});
