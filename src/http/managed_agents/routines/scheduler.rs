use std::{collections::BTreeSet, sync::Arc, time::Duration};

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

use crate::{
    db::managed_agents::{
        now_ms,
        routines::{repository, schema::RoutineRow},
    },
    errors::GatewayError,
    proxy::state::AppState,
};

const POLL_INTERVAL: Duration = Duration::from_secs(60);
const BACKFILL_WINDOW_MS: i64 = 32 * 24 * 60 * 60 * 1000;
const MINUTE_MS: i64 = 60 * 1000;

pub fn spawn(state: Arc<AppState>) {
    if state.db.is_none() {
        return;
    }
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = run_due_once(state.clone()).await {
                tracing::warn!("routine scheduler tick failed: {error}");
            }
        }
    });
}

pub async fn run_due_once(state: Arc<AppState>) -> Result<usize, GatewayError> {
    let Some(pool) = state.db.as_ref().cloned() else {
        return Ok(0);
    };
    let now = now_ms();
    let routines = repository::list_active(&pool).await?;
    let mut triggered = 0;
    for routine in routines {
        if !is_due(&routine, now) {
            continue;
        }
        match super::trigger::trigger_routine_run(
            state.clone(),
            pool.clone(),
            &routine.id,
            "localhost",
        )
        .await
        {
            Ok(run) => {
                triggered += 1;
                tracing::info!(
                    routine_id = %routine.id,
                    agent_id = %routine.agent_id,
                    run_id = %run.run_id,
                    "triggered scheduled routine"
                );
            }
            Err(error) => {
                tracing::warn!(
                    routine_id = %routine.id,
                    agent_id = %routine.agent_id,
                    "scheduled routine trigger failed: {error}"
                );
            }
        }
    }
    Ok(triggered)
}

fn is_due(routine: &RoutineRow, now: i64) -> bool {
    if routine.status != "active" {
        return false;
    }
    latest_scheduled_at(&routine.cron, &routine.timezone, threshold(routine), now).is_some()
}

fn threshold(routine: &RoutineRow) -> i64 {
    routine.last_run_at.unwrap_or(routine.created_at)
}

fn latest_scheduled_at(cron: &str, timezone: &str, threshold: i64, now: i64) -> Option<i64> {
    let schedule = CronSchedule::parse(cron)?;
    let tz = timezone.parse::<Tz>().ok()?;
    let mut cursor = utc_minute(now)?;
    let earliest = threshold.max(now.saturating_sub(BACKFILL_WINDOW_MS));
    while cursor.timestamp_millis() > earliest {
        if schedule.matches(&cursor.with_timezone(&tz)) {
            return Some(cursor.timestamp_millis());
        }
        cursor -= chrono::Duration::minutes(1);
    }
    None
}

fn utc_minute(ms: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_millis_opt(ms - ms.rem_euclid(MINUTE_MS))
        .single()
}

#[derive(Debug)]
struct CronSchedule {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
}

impl CronSchedule {
    fn parse(expr: &str) -> Option<Self> {
        let parts = expr.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 5 {
            return None;
        }
        Some(Self {
            minute: CronField::parse(parts[0], 0, 59, normalize_identity)?,
            hour: CronField::parse(parts[1], 0, 23, normalize_identity)?,
            day_of_month: CronField::parse(parts[2], 1, 31, normalize_identity)?,
            month: CronField::parse(parts[3], 1, 12, normalize_identity)?,
            day_of_week: CronField::parse(parts[4], 0, 7, normalize_day_of_week)?,
        })
    }

    fn matches(&self, local: &DateTime<Tz>) -> bool {
        self.minute.contains(local.minute())
            && self.hour.contains(local.hour())
            && self.day_of_month.contains(local.day())
            && self.month.contains(local.month())
            && self
                .day_of_week
                .contains(local.weekday().num_days_from_sunday())
    }
}

#[derive(Debug)]
struct CronField {
    values: BTreeSet<u32>,
}

impl CronField {
    fn parse(raw: &str, min: u32, max: u32, normalize: fn(u32) -> Option<u32>) -> Option<Self> {
        let mut values = BTreeSet::new();
        for part in raw.split(',') {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            let (range, step) = split_step(part)?;
            let (start, end) = parse_range(range, min, max)?;
            if step == 0 || start > end {
                return None;
            }
            let mut value = start;
            while value <= end {
                values.insert(normalize(value)?);
                value = value.saturating_add(step);
                if step == 0 {
                    return None;
                }
            }
        }
        (!values.is_empty()).then_some(Self { values })
    }

    fn contains(&self, value: u32) -> bool {
        self.values.contains(&value)
    }
}

fn split_step(part: &str) -> Option<(&str, u32)> {
    let mut pieces = part.split('/');
    let range = pieces.next()?;
    let step = match pieces.next() {
        Some(raw) => raw.parse().ok()?,
        None => 1,
    };
    if pieces.next().is_some() {
        return None;
    }
    Some((range, step))
}

fn parse_range(raw: &str, min: u32, max: u32) -> Option<(u32, u32)> {
    if raw == "*" || raw == "?" {
        return Some((min, max));
    }
    if let Some((start, end)) = raw.split_once('-') {
        return Some((parse_value(start, min, max)?, parse_value(end, min, max)?));
    }
    let value = parse_value(raw, min, max)?;
    Some((value, value))
}

fn parse_value(raw: &str, min: u32, max: u32) -> Option<u32> {
    let value = raw.parse::<u32>().ok()?;
    (min..=max).contains(&value).then_some(value)
}

fn normalize_identity(value: u32) -> Option<u32> {
    Some(value)
}

fn normalize_day_of_week(value: u32) -> Option<u32> {
    if value == 7 {
        Some(0)
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routine(cron: &str, created_at: i64, last_run_at: Option<i64>) -> RoutineRow {
        RoutineRow {
            id: "routine_test".to_owned(),
            agent_id: "agent_test".to_owned(),
            name: "Routine".to_owned(),
            prompt: "Run".to_owned(),
            cron: cron.to_owned(),
            timezone: "America/Los_Angeles".to_owned(),
            status: "active".to_owned(),
            last_run_id: None,
            last_run_at,
            created_at,
            updated_at: created_at,
        }
    }

    fn la_ms(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> i64 {
        chrono_tz::America::Los_Angeles
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn daily_routine_is_due_after_scheduled_minute() {
        let row = routine("0 9 * * *", la_ms(2026, 6, 15, 8, 0), None);

        assert!(is_due(&row, la_ms(2026, 6, 15, 9, 5)));
    }

    #[test]
    fn routine_created_after_scheduled_minute_waits_until_next_match() {
        let row = routine("0 9 * * *", la_ms(2026, 6, 15, 10, 0), None);

        assert!(!is_due(&row, la_ms(2026, 6, 15, 11, 0)));
    }

    #[test]
    fn routine_does_not_repeat_same_scheduled_minute() {
        let row = routine(
            "0 9 * * *",
            la_ms(2026, 6, 15, 8, 0),
            Some(la_ms(2026, 6, 15, 9, 0)),
        );

        assert!(!is_due(&row, la_ms(2026, 6, 15, 9, 30)));
    }

    #[test]
    fn weekday_range_matches_monday_but_not_sunday() {
        let monday = routine("0 9 * * 1-5", la_ms(2026, 6, 15, 8, 0), None);
        let sunday = routine("0 9 * * 1-5", la_ms(2026, 6, 14, 8, 0), None);

        assert!(is_due(&monday, la_ms(2026, 6, 15, 9, 1)));
        assert!(!is_due(&sunday, la_ms(2026, 6, 14, 9, 1)));
    }

    #[test]
    fn stepped_minute_cron_is_supported() {
        let row = routine("*/15 * * * *", la_ms(2026, 6, 15, 8, 1), None);

        assert!(is_due(&row, la_ms(2026, 6, 15, 8, 30)));
    }
}
