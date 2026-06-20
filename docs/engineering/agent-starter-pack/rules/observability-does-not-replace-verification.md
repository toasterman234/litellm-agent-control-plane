---
name: Observability Does Not Replace Verification
description: Use logs, traces, and dashboards for insight, but do not treat them as a substitute for verification or user-level evidence.
---

# Observability Does Not Replace Verification

## Rule

Logs, traces, metrics, and dashboards are useful evidence sources, but they do not by themselves prove that the user-facing outcome is correct.

## Apply this when

- debugging systems
- reviewing production behavior
- reporting completion
- interpreting traces or dashboards

## Required behavior

- Use observability tools to narrow the problem.
- Verify the real outcome at the behavior level when possible.
- Distinguish between "the service emitted the expected telemetry" and "the feature worked for the user".
- If only telemetry was checked, say so explicitly.

## Good

- "The trace shows the request reached the provider, and the end-to-end reproduction now succeeds."
- "Metrics look healthy, but the user flow was not re-run."

## Bad

- "The dashboard looks green, so it works."
- "The trace exists, therefore the bug is fixed."
