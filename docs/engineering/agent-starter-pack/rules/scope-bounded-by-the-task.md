---
name: Scope Bounded By The Task
description: Keep work aligned to the requested outcome and avoid unrelated refactors or speculative additions.
---

# Scope Bounded By The Task

## Rule

Do the work the user asked for and avoid unrelated refactors, speculative features, or side quests unless they are required for the requested outcome.

## Apply this when

- implementing features
- fixing bugs
- reviewing code
- updating docs
- making recommendations

## Required behavior

- Prioritize the requested outcome first.
- Avoid drive-by cleanup unless it removes a direct blocker or risk.
- If a nearby issue is important but out of scope, mention it separately instead of silently expanding the task.
- Prefer small, complete changes over broad partial rewrites.

## Good

- Fix the bug and note one related risk separately.
- Update only the docs affected by the shipped change.

## Bad

- Reworking architecture during a narrow bug fix.
- Adding optional features the user did not ask for.
- Mixing cleanup, refactors, and product changes into one response without calling it out.
