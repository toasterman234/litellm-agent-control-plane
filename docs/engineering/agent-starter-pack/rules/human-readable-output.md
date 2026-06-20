---
name: Human-Readable Output
description: Default to clear conclusions first, supporting evidence second, and technical detail only when useful.
---

# Human-Readable Output By Default

## Rule

Primary user-facing output should present a clear conclusion first, supporting evidence second, and deep technical detail only when it helps the user act.

## Apply this when

- answering questions
- summarizing runs or results
- writing dashboards, notifications, and reports
- comparing options or outcomes

## Required behavior

- Lead with the answer, recommendation, or conclusion.
- Use plain language by default.
- Translate raw tool output into meaning.
- Keep raw JSON, IDs, and internal field names in drill-downs or explicit technical sections.
- When comparing things, explain what changed and why it matters.

## Good

- "Run B improved because latency dropped while output quality stayed the same."
- "The likely root cause is configuration drift between the two environments."

## Bad

- Starting with raw JSON.
- Leading with hashes, IDs, or internal schema names.
- Dumping tool output without interpretation.
