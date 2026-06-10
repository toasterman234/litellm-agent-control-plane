import assert from "node:assert/strict";
import test from "node:test";

import { modelListFromIds, modelListFromValue } from "../src/model-list.mjs";

test("modelListFromIds returns OpenAI-shaped models", () => {
  assert.deepEqual(modelListFromIds([" gpt-5.5 ", "", "claude-sonnet-4-6"], "litellm"), {
    object: "list",
    data: [
      { id: "gpt-5.5", object: "model", created: 0, owned_by: "litellm" },
      { id: "claude-sonnet-4-6", object: "model", created: 0, owned_by: "litellm" },
    ],
  });
});

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
