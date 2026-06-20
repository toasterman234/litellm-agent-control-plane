---
name: Automation Requires Action
description: Only create automations, reports, or checks when there is a named consumer and a defined follow-up action.
---

# Automation Requires Action

## Rule

Do not create an automated check, report, digest, routine, monitor, or analysis step unless there is a named consumer and a defined follow-up action for its output.

## Apply this when

- proposing routines or scheduled jobs
- adding reports or dashboards
- designing alerts or monitors
- recommending workflow automation

## Required behavior

- Identify who receives or reads the output.
- Identify what happens when the output matters.
- If either is missing, treat the automation as incomplete.
- Prefer closed loops over passive output.

## Good

- "This routine posts a daily summary to the team channel, and failures create an inbox item for triage."
- "This report is reviewed weekly by the ops lead and drives backlog prioritization."

## Bad

- A dashboard nobody checks.
- A scheduled report that only writes to disk.
- An alert with no owner.
