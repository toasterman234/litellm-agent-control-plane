/**
 * scripts/cron-parse.test.mjs
 *
 * Unit tests for the cron-spec parsing helpers in src/server/cron.ts.
 *
 * Run with:
 *   node --import tsx --test scripts/cron-parse.test.mjs
 *
 * What we cover:
 *   - parseCronSpec returns null for empty/missing schedules
 *   - parseCronSpec computes a sensible "next" instant for a valid schedule
 *   - parseCronSpec throws a useful Error for bad cron strings + bad tz
 *   - computeNextFireAt respects the `after` argument
 *   - computeNextFireAt produces different instants for different timezones
 *
 * We don't exercise the worker tick here — that needs a live Postgres
 * with FOR UPDATE SKIP LOCKED semantics, covered by integration tests.
 *
 * The cron module is TypeScript; we run via tsx (already a devDep) to
 * load it directly without a separate build step.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

// tsx ESM hook is registered via `node --import tsx` (see header comment).
const { parseCronSpec, computeNextFireAt } = await import(
  "../src/server/cron.ts"
);

describe("parseCronSpec", () => {
  it("returns next=null for empty/null schedule", () => {
    assert.equal(parseCronSpec("", "UTC").next, null);
    assert.equal(parseCronSpec(null, "UTC").next, null);
    assert.equal(parseCronSpec(undefined, "UTC").next, null);
    assert.equal(parseCronSpec("   ", "UTC").next, null);
  });

  it("computes next fire for a simple daily schedule (UTC)", () => {
    const now = new Date("2026-05-18T08:00:00Z");
    // Override "now" via computeNextFireAt — parseCronSpec uses its own
    // internal clock, so we only assert the result is in the future and
    // matches the cadence.
    const next = computeNextFireAt("0 9 * * *", "UTC", now);
    assert.ok(next instanceof Date);
    assert.equal(next.getUTCHours(), 9);
    assert.equal(next.getUTCMinutes(), 0);
    // Should be the same day since we're at 08:00 UTC and 09:00 UTC has not passed.
    assert.equal(next.toISOString(), "2026-05-18T09:00:00.000Z");
  });

  it("rolls to next day when today's slot has passed", () => {
    const now = new Date("2026-05-18T10:00:00Z");
    const next = computeNextFireAt("0 9 * * *", "UTC", now);
    assert.equal(next.toISOString(), "2026-05-19T09:00:00.000Z");
  });

  it("respects timezone: 0 9 * * * in America/Los_Angeles is 16:00 or 17:00 UTC", () => {
    // We don't pin DST: just assert the UTC hour is in the LA->UTC offset
    // range (15..17 absorbs PST/PDT plus the dateline).
    const after = new Date("2026-05-18T00:00:00Z");
    const next = computeNextFireAt("0 9 * * *", "America/Los_Angeles", after);
    const h = next.getUTCHours();
    assert.ok(h === 16 || h === 17, `expected UTC hour 16 or 17, got ${h}`);
  });

  it("throws on invalid cron string", () => {
    assert.throws(
      () => parseCronSpec("not a cron expression", "UTC"),
      /invalid cron_schedule/,
    );
  });

  it("throws on invalid timezone", () => {
    assert.throws(
      () => parseCronSpec("0 9 * * *", "Invalid/Timezone"),
      /invalid cron_timezone/,
    );
  });
});

describe("computeNextFireAt", () => {
  it("uses the `after` argument as the reference point", () => {
    const a = computeNextFireAt(
      "*/5 * * * *",
      "UTC",
      new Date("2026-05-18T08:00:00Z"),
    );
    const b = computeNextFireAt(
      "*/5 * * * *",
      "UTC",
      new Date("2026-05-18T09:00:00Z"),
    );
    assert.ok(b.getTime() > a.getTime(), "later `after` should yield later next");
  });

  it("every-5-minutes lands on a 5-minute boundary", () => {
    const next = computeNextFireAt(
      "*/5 * * * *",
      "UTC",
      new Date("2026-05-18T08:01:00Z"),
    );
    assert.equal(next.getUTCMinutes() % 5, 0);
    // First 5-minute mark after 08:01 is 08:05.
    assert.equal(next.toISOString(), "2026-05-18T08:05:00.000Z");
  });

  it("weekday-only schedule skips weekends", () => {
    // 2026-05-16 is a Saturday — next weekday fire of `0 9 * * 1-5` is Monday.
    const after = new Date("2026-05-16T08:00:00Z");
    const next = computeNextFireAt("0 9 * * 1-5", "UTC", after);
    assert.equal(next.toISOString(), "2026-05-18T09:00:00.000Z");
  });
});
