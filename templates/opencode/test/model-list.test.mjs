import assert from "node:assert/strict";
import test from "node:test";

import { modelListFromLines, modelListFromValue } from "../src/model-list.mjs";

test("modelListFromValue accepts OpenAI and Gemini-style payloads", () => {
  assert.deepEqual(modelListFromValue({
    data: [
      { id: "gpt-5.5", object: "model", created: 123, owned_by: "openai" },
    ],
  }, "litellm"), {
    object: "list",
    data: [
      { id: "gpt-5.5", object: "model", created: 123, owned_by: "openai" },
    ],
  });

  assert.deepEqual(modelListFromValue({
    models: [
      { name: "models/gemini-2.5-pro" },
    ],
  }, "gemini"), {
    object: "list",
    data: [
      { id: "gemini-2.5-pro", object: "model", created: 0, owned_by: "gemini" },
    ],
  });
});

test("modelListFromLines accepts opencode provider model output", () => {
  assert.deepEqual(
    modelListFromLines("opencode-go/qwen3.7-max\nopencode-go/qwen3.7-plus\n", "opencode-go"),
    {
      object: "list",
      data: [
        { id: "opencode-go/qwen3.7-max", object: "model", created: 0, owned_by: "opencode-go" },
        { id: "opencode-go/qwen3.7-plus", object: "model", created: 0, owned_by: "opencode-go" },
      ],
    }
  );
});
