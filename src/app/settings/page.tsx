"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { RefreshCw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { AdminStats, ApiError, getAdminStats } from "@/lib/api";

const POLL_INTERVAL_MS = 5000;

function formatRelative(iso?: string | null): string {
  if (!iso) return "—";
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return iso;
  const diff = Date.now() - then;
  if (diff < 0) return "just now";
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.floor(hr / 24)}d ago`;
}

function maskArn(arn: string): string {
  // Task definition ARNs include the AWS account ID. Show enough to
  // identify which task def revision is wired up without leaking it.
  if (arn.length < 24) return arn;
  return `${arn.slice(0, 16)}…${arn.slice(-12)}`;
}

interface StatCardProps {
  label: string;
  value: string | number;
  hint?: string;
  tone?: "default" | "warn" | "ok";
}

function StatCard({ label, value, hint, tone = "default" }: StatCardProps) {
  const toneClass =
    tone === "warn"
      ? "text-amber-600 dark:text-amber-500"
      : tone === "ok"
        ? "text-emerald-600 dark:text-emerald-500"
        : "text-foreground";
  return (
    <div className="rounded-md border border-border bg-card p-4">
      <div className="text-xs uppercase tracking-wide text-muted-foreground">
        {label}
      </div>
      <div className={`mt-1 text-2xl font-semibold tabular-nums ${toneClass}`}>
        {value}
      </div>
      {hint ? (
        <div className="mt-1 text-xs text-muted-foreground">{hint}</div>
      ) : null}
    </div>
  );
}

export default function SettingsPage() {
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const load = useCallback(async (background = false) => {
    if (!background) setLoading(true);
    setRefreshing(true);
    setError(null);
    try {
      const next = await getAdminStats();
      setStats(next);
    } catch (e) {
      const msg =
        e instanceof ApiError
          ? `${e.status} ${e.message}`
          : e instanceof Error
            ? e.message
            : "failed to load stats";
      setError(msg);
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    load();
    const interval = setInterval(() => load(true), POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [load]);

  const wp = stats?.warm_pool;
  const sx = stats?.sessions;
  const rt = stats?.runtime;

  // Pool depth at a glance — color the headline number based on whether
  // we're at target, partially provisioned, or empty.
  const poolHealthTone: StatCardProps["tone"] = !wp
    ? "default"
    : wp.counts.warm >= wp.configured_size
      ? "ok"
      : wp.counts.warm + wp.counts.provisioning >= wp.configured_size
        ? "default"
        : "warn";

  return (
    <div className="mx-auto w-full max-w-5xl px-4 py-8 sm:px-6">
      <div className="mb-6 flex items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold">Settings</h1>
          <p className="text-sm text-muted-foreground">
            Live snapshot of the warm pool and active sandboxes. Polls every
            5s.
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => load()}
          disabled={refreshing}
        >
          <RefreshCw
            className={`mr-2 h-3.5 w-3.5 ${refreshing ? "animate-spin" : ""}`}
          />
          Refresh
        </Button>
      </div>

      {error ? (
        <div className="mb-6 rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      ) : null}

      {loading && !stats ? (
        <div className="text-sm text-muted-foreground">Loading…</div>
      ) : null}

      {stats ? (
        <div className="space-y-8">
          {/* Warm pool */}
          <section>
            <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
              Warm pool
            </h2>
            <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
              <StatCard
                label="Warm (ready to claim)"
                value={wp?.counts.warm ?? 0}
                hint={`target ${wp?.configured_size ?? 0}`}
                tone={poolHealthTone}
              />
              <StatCard
                label="Provisioning"
                value={wp?.counts.provisioning ?? 0}
                hint={`max concurrent ${wp?.max_provisioning ?? 0}`}
              />
              <StatCard
                label="Claimed (in-flight)"
                value={wp?.counts.claimed ?? 0}
                hint="brief — usually 0"
              />
              <StatCard
                label="Dead (cleanup pending)"
                value={wp?.counts.dead ?? 0}
                hint="reaped each tick"
                tone={(wp?.counts.dead ?? 0) > 5 ? "warn" : "default"}
              />
            </div>
            <div className="mt-3 grid grid-cols-2 gap-3 md:grid-cols-4 text-xs text-muted-foreground">
              <div>
                <span className="font-mono">WARM_POOL_SIZE</span> ={" "}
                <span className="text-foreground">
                  {wp?.configured_size ?? 0}
                </span>
              </div>
              <div>
                <span className="font-mono">MAX_PROVISIONING</span> ={" "}
                <span className="text-foreground">
                  {wp?.max_provisioning ?? 0}
                </span>
              </div>
              <div>
                <span className="font-mono">TTL_MINUTES</span> ={" "}
                <span className="text-foreground">{wp?.ttl_minutes ?? 0}</span>
              </div>
              <div>
                <span className="font-mono">RECENT_AGENT_HOURS</span> ={" "}
                <span className="text-foreground">
                  {wp?.recent_agent_hours ?? 0}
                </span>
              </div>
            </div>

            {wp && wp.by_agent.length > 0 ? (
              <div className="mt-4 overflow-x-auto rounded-md border border-border">
                <table className="w-full text-sm">
                  <thead className="bg-muted/40 text-xs uppercase text-muted-foreground">
                    <tr>
                      <th className="px-3 py-2 text-left font-medium">
                        Agent
                      </th>
                      <th className="px-3 py-2 text-right font-medium">
                        Warm
                      </th>
                      <th className="px-3 py-2 text-right font-medium">
                        Provisioning
                      </th>
                      <th className="px-3 py-2 text-right font-medium">
                        Claimed
                      </th>
                      <th className="px-3 py-2 text-right font-medium">
                        Dead
                      </th>
                      <th className="px-3 py-2 text-left font-medium">
                        Oldest warm
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {wp.by_agent.map((row) => (
                      <tr key={row.agent_id} className="border-t border-border">
                        <td className="px-3 py-2">
                          <Link
                            href={`/agents/${row.agent_id}`}
                            className="hover:underline"
                          >
                            {row.agent_name ?? (
                              <span className="font-mono text-xs text-muted-foreground">
                                {row.agent_id.slice(0, 8)}
                              </span>
                            )}
                          </Link>
                        </td>
                        <td className="px-3 py-2 text-right tabular-nums">
                          {row.warm}
                        </td>
                        <td className="px-3 py-2 text-right tabular-nums text-muted-foreground">
                          {row.provisioning}
                        </td>
                        <td className="px-3 py-2 text-right tabular-nums text-muted-foreground">
                          {row.claimed}
                        </td>
                        <td className="px-3 py-2 text-right tabular-nums text-muted-foreground">
                          {row.dead}
                        </td>
                        <td className="px-3 py-2 text-xs text-muted-foreground">
                          {formatRelative(row.oldest_warm_at)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p className="mt-4 text-sm text-muted-foreground">
                No warm tasks. Either{" "}
                <span className="font-mono">WARM_POOL_SIZE=0</span>, no agent
                has been used in the last{" "}
                {wp?.recent_agent_hours ?? 0}h, or the pool just turned over.
              </p>
            )}
          </section>

          {/* Sessions */}
          <section>
            <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
              Sandboxes (live sessions)
            </h2>
            <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
              <StatCard
                label="Ready"
                value={sx?.counts.ready ?? 0}
                hint="serving requests"
                tone="ok"
              />
              <StatCard
                label="Creating"
                value={sx?.counts.creating ?? 0}
                hint="still booting"
              />
              <StatCard
                label="Failed"
                value={sx?.counts.failed ?? 0}
                hint="terminal"
                tone={(sx?.counts.failed ?? 0) > 0 ? "warn" : "default"}
              />
              <StatCard
                label="Dead"
                value={sx?.counts.dead ?? 0}
                hint="cleaned up"
              />
            </div>

            {sx && sx.by_agent.length > 0 ? (
              <div className="mt-4 overflow-x-auto rounded-md border border-border">
                <table className="w-full text-sm">
                  <thead className="bg-muted/40 text-xs uppercase text-muted-foreground">
                    <tr>
                      <th className="px-3 py-2 text-left font-medium">
                        Agent
                      </th>
                      <th className="px-3 py-2 text-right font-medium">
                        Ready
                      </th>
                      <th className="px-3 py-2 text-right font-medium">
                        Creating
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {sx.by_agent.map((row) => (
                      <tr key={row.agent_id} className="border-t border-border">
                        <td className="px-3 py-2">
                          <Link
                            href={`/agents/${row.agent_id}`}
                            className="hover:underline"
                          >
                            {row.agent_name ?? (
                              <span className="font-mono text-xs text-muted-foreground">
                                {row.agent_id.slice(0, 8)}
                              </span>
                            )}
                          </Link>
                        </td>
                        <td className="px-3 py-2 text-right tabular-nums">
                          {row.ready}
                        </td>
                        <td className="px-3 py-2 text-right tabular-nums text-muted-foreground">
                          {row.creating}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <p className="mt-4 text-sm text-muted-foreground">
                No live sessions.
              </p>
            )}
          </section>

          {/* Runtime */}
          <section>
            <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
              Runtime
            </h2>
            <div className="rounded-md border border-border bg-card divide-y divide-border text-sm">
              <Row label="Region">{rt?.aws_region ?? "—"}</Row>
              <Row label="ECS cluster">{rt?.aws_cluster ?? "—"}</Row>
              <Row label="Task definition">
                <span className="font-mono text-xs">
                  {rt ? maskArn(rt.task_definition_arn) : "—"}
                </span>
              </Row>
              <Row label="Container port">
                {rt?.container_port ?? "—"}
              </Row>
              <Row label="Reconcile interval">
                {rt?.reconcile_interval_seconds ?? 0}s
              </Row>
              <Row label="Total agents">{stats.agents.total}</Row>
            </div>
          </section>

          <section className="text-xs text-muted-foreground">
            <p>
              Knobs are read from process env at boot. To change them, update
              the env vars on the host (Render dashboard, <code>.env</code>{" "}
              for local) and restart the service. See{" "}
              <code>src/server/warmPool/README.md</code> for sizing guidance.
            </p>
          </section>
        </div>
      ) : null}
    </div>
  );
}

function Row({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-3 px-4 py-3">
      <div className="w-44 shrink-0 text-muted-foreground">{label}</div>
      <div className="min-w-0 flex-1 break-all">{children}</div>
    </div>
  );
}
