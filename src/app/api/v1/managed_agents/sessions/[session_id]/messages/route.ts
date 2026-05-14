/**
 * GET /api/v1/managed_agents/sessions/[session_id]/messages
 *
 * Returns the full thread for a session as an array of `HarnessMessage`
 * ({info, parts}). Reads from the append-only `session_event` log first
 * (always available, no harness round-trip needed) so a different browser
 * or a dead pod doesn't blind the UI.
 *
 * For live in-flight turns we also peek at the harness — if the harness
 * has more messages than we've persisted yet (the async event-log writer
 * in /message hasn't run yet, or the harness emitted a streaming
 * intermediate we don't store), prefer the harness view. This keeps the
 * UI feeling instant while the DB catches up.
 *
 * Wire shape is unchanged — callers continue to read
 * `HarnessMessage[]`.
 */

import { assertAuth } from "@/server/auth";
import { prisma } from "@/server/db";
import { harnessListMessages } from "@/server/harness";
import { listEvents } from "@/server/sessionEvents";
import { HttpError, httpError } from "@/server/types";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

interface RouteContext {
  params: Promise<{ session_id: string }>;
}

export async function GET(req: Request, ctx: RouteContext) {
  try {
    assertAuth(req);
    const { session_id } = await ctx.params;

    const row = await prisma.session.findUnique({
      where: { session_id },
      select: { sandbox_url: true, harness_session_id: true, status: true },
    });
    if (!row) httpError(404, `session ${session_id} not found`);

    // Always read the persisted log first. Works regardless of pod state
    // (including `dead` rows, which is the whole point of the new table).
    const persisted = await listEvents(session_id);

    // If the pod is alive, opportunistically check whether the harness has
    // newer messages than we've persisted (the async writer may still be
    // in-flight after the prior /message call). If yes, return that — the
    // UI gets the freshest view while the DB catches up. If the harness
    // call fails, fall back to the persisted log silently — we still have
    // a complete history.
    if (
      row.status === "ready" &&
      row.sandbox_url &&
      row.harness_session_id
    ) {
      try {
        const live = await harnessListMessages({
          sandbox_url: row.sandbox_url,
          harness_session_id: row.harness_session_id,
        });
        if (live.length > persisted.length) return Response.json(live);
      } catch (err) {
        // Harness blip — persisted log is authoritative.
        console.warn(
          `harness list_messages failed for session ${session_id}; serving persisted log:`,
          err,
        );
      }
    }

    return Response.json(persisted);
  } catch (e) {
    if (e instanceof Response) return e;
    if (e instanceof HttpError)
      return Response.json({ error: e.detail }, { status: e.status });
    console.error(e);
    return Response.json({ error: "internal error" }, { status: 500 });
  }
}
