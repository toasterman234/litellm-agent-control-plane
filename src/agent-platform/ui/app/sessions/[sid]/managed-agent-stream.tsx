"use client";

import { useCallback, useEffect, useRef, useState } from "react";

import type {
  AgentMessage,
  AgentPart,
  PermissionRequest,
} from "@/shared/agent-state";
import {
  getSessionThread,
  sendMessage,
  subscribeManagedAgentEvents,
  type ManagedAgentEvent,
  type SendMessageAttachment,
} from "@/ui/lib/api";
import type { SendParts, PermissionResponse } from "./opencode-stream";

export interface ManagedAgentThread {
  messages: AgentMessage[];
  subThreads: Map<string, AgentMessage[]>;
  permissions: PermissionRequest[];
  busy: boolean;
  error?: string;
  send: (
    parts: SendParts,
    model?: { providerID: string; modelID: string },
  ) => Promise<void>;
  respondPermission: (
    permissionID: string,
    permSessionID: string,
    response: PermissionResponse,
  ) => Promise<void>;
}

function eventText(ev: ManagedAgentEvent): string {
  return (ev.content ?? [])
    .filter((block) => block.type === "text" && typeof block.text === "string")
    .map((block) => block.text)
    .join("");
}

function eventImages(ev: ManagedAgentEvent): AgentPart[] {
  return (ev.content ?? [])
    .filter((block) => block.type === "image")
    .map((block, index) => ({
      ...block,
      id: `${ev.id ?? "image"}-${index}`,
      type: "image",
    }));
}

function upsertMessage(
  messages: AgentMessage[],
  next: AgentMessage,
): AgentMessage[] {
  const idx = messages.findIndex((m) => m.id === next.id);
  if (idx < 0) return [...messages, next];
  return messages.map((m, i) => (i === idx ? next : m));
}

function updateToolResult(
  messages: AgentMessage[],
  ev: ManagedAgentEvent,
): AgentMessage[] {
  const toolUseId = ev.tool_use_id;
  if (!toolUseId) return messages;
  let matched = false;
  const next = messages.map((message) => {
    if (message.role !== "assistant") return message;
    const parts = message.parts.map((part) => {
      if (part.callID !== toolUseId && part.id !== toolUseId) return part;
      matched = true;
      return {
        ...part,
        state: {
          ...(part.state ?? {}),
          status: ev.is_error ? "error" : "completed",
          output:
            typeof ev.content === "string"
              ? ev.content
              : JSON.stringify(ev.content ?? ""),
          error: ev.is_error ? String(ev.content ?? "tool error") : undefined,
        },
      };
    });
    return { ...message, parts };
  });
  if (matched) return next;
  return [
    ...messages,
    {
      id: ev.id ?? `${toolUseId}-result`,
      role: "assistant",
      parts: [
        {
          id: toolUseId,
          type: "tool",
          callID: toolUseId,
          state: {
            status: ev.is_error ? "error" : "completed",
            output: JSON.stringify(ev.content ?? ""),
          },
        },
      ],
    },
  ];
}

function applyManagedEvent(messages: AgentMessage[], ev: ManagedAgentEvent): AgentMessage[] {
  const id = ev.id ?? `${ev.type}-${messages.length}`;
  switch (ev.type) {
    case "user.message": {
      const text = eventText(ev);
      return upsertMessage(messages, {
        id,
        role: "user",
        parts: [
          ...(text ? [{ id: `${id}-text`, type: "text", text }] : []),
          ...eventImages(ev),
        ],
      });
    }
    case "agent.message": {
      const text = eventText(ev);
      if (!text) return messages;
      return upsertMessage(messages, {
        id,
        role: "assistant",
        parts: [{ id: `${id}-text`, type: "text", text }],
      });
    }
    case "agent.tool_use": {
      const toolUseId = ev.tool_use_id ?? id;
      return upsertMessage(messages, {
        id,
        role: "assistant",
        parts: [
          {
            id: toolUseId,
            type: "tool",
            tool: ev.name ?? "tool",
            callID: toolUseId,
            state: {
              input: ev.input,
              status: "running",
            },
          },
        ],
      });
    }
    case "agent.tool_result":
      return updateToolResult(messages, ev);
    default:
      return messages;
  }
}

function attachmentFromDataUrl(url: string): SendMessageAttachment | null {
  const match = /^data:([^;,]+);base64,(.*)$/.exec(url);
  if (!match) return null;
  return { mime_type: match[1], base64: match[2] };
}

function partsToRequest(parts: SendParts): {
  text?: string;
  attachments?: SendMessageAttachment[];
} {
  const text = parts
    .filter((part) => part.type === "text")
    .map((part) => part.text)
    .join("");
  const attachments = parts
    .filter((part) => part.type === "file")
    .map((part) => attachmentFromDataUrl(part.url))
    .filter((part): part is SendMessageAttachment => Boolean(part));
  return {
    text: text || undefined,
    attachments: attachments.length > 0 ? attachments : undefined,
  };
}

export function useManagedAgentThread(
  sessionId: string,
  enabled: boolean,
): ManagedAgentThread {
  const [messages, setMessages] = useState<AgentMessage[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | undefined>();
  const seenRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    if (!enabled || !sessionId) return;
    let cancelled = false;
    seenRef.current = new Set();
    setMessages([]);
    setError(undefined);

    void (async () => {
      try {
        const history = (await getSessionThread(sessionId)) as ManagedAgentEvent[];
        if (cancelled) return;
        setMessages((prev) => {
          let next = prev;
          for (const ev of history) {
            if (ev.id) seenRef.current.add(ev.id);
            next = applyManagedEvent(next, ev);
          }
          return next;
        });
      } catch (err) {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err));
      }
    })();

    const es = subscribeManagedAgentEvents(sessionId, (ev) => {
      if (cancelled) return;
      if (ev.id && seenRef.current.has(ev.id)) return;
      if (ev.id) seenRef.current.add(ev.id);
      if (ev.type === "session.status_idle") {
        setBusy(false);
        return;
      }
      if (ev.type === "session.status_error") {
        setBusy(false);
        setError(ev.error ?? "harness session error");
        return;
      }
      setMessages((prev) => applyManagedEvent(prev, ev));
    });
    es.onerror = () => {
      if (!cancelled && es.readyState === EventSource.CLOSED) {
        setError("harness event stream closed");
      }
    };

    return () => {
      cancelled = true;
      es.close();
    };
  }, [sessionId, enabled]);

  const send = useCallback(
    async (parts: SendParts) => {
      if (!sessionId) throw new Error("session not ready");
      setBusy(true);
      setError(undefined);
      try {
        await sendMessage(sessionId, partsToRequest(parts));
      } catch (err) {
        setBusy(false);
        throw err;
      }
    },
    [sessionId],
  );

  const respondPermission = useCallback(async () => {
    throw new Error("permissions are not supported by the managed-agents harness server");
  }, []);

  return {
    messages,
    subThreads: new Map(),
    permissions: [],
    busy,
    error,
    send,
    respondPermission,
  };
}
