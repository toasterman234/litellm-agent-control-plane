/**
 * Claude-Agent-SDK adapter for the shared managed-tools/slack spec.
 *
 * All the real logic — schema, description, HTTP client — lives in
 * `@lap/managed-tools/slack`. This file's only job is to bridge that
 * spec to the Claude Agent SDK's tool API (`tool()` + `createSdkMcpServer`).
 *
 * The tool is ALWAYS registered — even when SLACK_BOT_TOKEN is unset. The env
 * is checked at call time: a missing token returns an actionable error so the
 * agent can tell the user "Slack isn't configured" instead of the tool silently
 * vanishing from its toolset (which makes the missing capability invisible and
 * impossible for the agent to explain).
 *
 * When a future harness wants Slack, it imports from `@lap/managed-tools/slack`
 * and writes its own ~30-line adapter the same way.
 */

import {
  createSdkMcpServer,
  tool,
  type McpSdkServerConfigWithInstance,
} from "@anthropic-ai/claude-agent-sdk";
import {
  callPostSlackMessage,
  slackEnvStatus,
  postSlackMessageDescription,
  postSlackMessageSchema,
  type PostSlackMessageInput,
} from "@lap/managed-tools/slack";

export function buildSlackMcpServer(): McpSdkServerConfigWithInstance {
  const postSlackMessage = tool(
    "post_slack_message",
    postSlackMessageDescription,
    postSlackMessageSchema,
    async (input: PostSlackMessageInput) => {
      // Resolve the env at call time, not registration time. If it's missing,
      // hand the agent a clear, actionable message rather than a generic
      // failure — and tell it not to retry, since the fix is operator-side.
      const status = slackEnvStatus();
      if (!status.env) {
        return {
          content: [
            {
              type: "text" as const,
              text:
                `Slack is not configured on this platform: missing ${status.missing.join(", ")}. ` +
                `Posting is disabled until an operator sets CONTAINER_ENV_SLACK_BOT_TOKEN ` +
                `(a Slack bot token, xoxb-...) on the platform. ` +
                `Tell the user this rather than retrying — the fix is operator-side.`,
            },
          ],
          isError: true,
        };
      }

      const out = await callPostSlackMessage(status.env, input);
      return {
        content: [{ type: "text" as const, text: out.text }],
        ...(out.isError && { isError: true }),
      };
    },
  );

  return createSdkMcpServer({
    name: "lap-slack",
    version: "0.1.0",
    tools: [postSlackMessage],
  });
}

export const SLACK_TOOL_NAMES = [
  "mcp__lap-slack__post_slack_message",
] as const;
