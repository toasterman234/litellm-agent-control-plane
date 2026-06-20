import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileP = promisify(execFile);

export function modelListFromValue(value, ownedBy = "litellm") {
  const items = Array.isArray(value?.data)
    ? value.data
    : Array.isArray(value?.models)
      ? value.models
      : null;
  if (!items) return null;

  const data = items
    .map((item) => {
      const id = typeof item?.id === "string"
        ? item.id
        : typeof item?.name === "string"
          ? item.name.replace(/^models\//, "")
          : "";
      if (!id.trim()) return null;
      return {
        id: id.trim(),
        object: "model",
        created: Number.isFinite(item?.created) ? item.created : 0,
        owned_by: typeof item?.owned_by === "string" && item.owned_by.trim()
          ? item.owned_by
          : ownedBy,
      };
    })
    .filter(Boolean);

  return { object: "list", data };
}

export function modelListFromLines(text, ownedBy) {
  const data = String(text || "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((id) => ({
      id,
      object: "model",
      created: 0,
      owned_by: ownedBy,
    }));

  return { object: "list", data };
}

export async function fetchLiteLlmModels({ baseURL, apiKey, ownedBy = "litellm" }) {
  const url = new URL("models", `${baseURL.replace(/\/+$/, "")}/`);
  const res = await fetch(url, {
    headers: {
      authorization: `Bearer ${apiKey}`,
      accept: "application/json",
    },
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`LiteLLM models failed (${res.status}): ${body.slice(0, 500)}`);
  }

  const raw = await res.json().catch(() => null);
  const models = modelListFromValue(raw, ownedBy);
  if (!models) throw new Error("LiteLLM models response missing data");
  return models;
}

export async function fetchOpencodeProviderModels({ providerID, opencodeBin = "opencode" }) {
  const { stdout } = await execFileP(opencodeBin, ["models", providerID], {
    env: process.env,
    maxBuffer: 1024 * 1024,
  });
  return modelListFromLines(stdout, providerID);
}
