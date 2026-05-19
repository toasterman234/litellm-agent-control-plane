-- AlterTable: scheduled-trigger fields on Agent
ALTER TABLE "managed_agent"
  ADD COLUMN "cron_schedule"       TEXT,
  ADD COLUMN "cron_timezone"       TEXT      NOT NULL DEFAULT 'UTC',
  ADD COLUMN "cron_enabled"        BOOLEAN   NOT NULL DEFAULT true,
  ADD COLUMN "cron_overlap_policy" TEXT      NOT NULL DEFAULT 'skip',
  ADD COLUMN "cron_last_fired_at"  TIMESTAMP(3),
  ADD COLUMN "cron_next_fire_at"   TIMESTAMP(3);

-- AlterTable: tag sessions with what started them ("api" | "cron")
ALTER TABLE "managed_agent_session"
  ADD COLUMN "trigger" TEXT NOT NULL DEFAULT 'api';

-- CreateIndex: scheduler hot path — cheapest scan over due-and-enabled agents.
CREATE INDEX "managed_agent_cron_enabled_next_fire_at_idx"
  ON "managed_agent" ("cron_enabled", "cron_next_fire_at");
