import { registry } from "@/server/metrics";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

export function GET() {
  return new Response(registry.renderText(), {
    headers: { "Content-Type": "text/plain; version=0.0.4; charset=utf-8" },
  });
}
