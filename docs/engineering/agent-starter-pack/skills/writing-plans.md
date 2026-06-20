---
name: Writing Plans
description: Turn multi-step engineering work into a concrete implementation plan before touching code.
---

# Writing Plans

Use this skill to create a concrete implementation plan for multi-step work.

## Use this skill when

- the user asks for a plan
- the task is too large for one safe pass
- the work spans multiple files or subsystems
- execution order matters

## Workflow

1. Define the goal in one sentence.
2. Map the files or systems likely to change.
3. Break the work into small, reviewable steps.
4. Include validation or acceptance checks.
5. Call out risky decisions, dependencies, and open questions.

## Required behavior

- Keep the plan concrete.
- Prefer steps that produce working increments.
- Include verification, not just implementation.
- Avoid vague placeholders like "handle edge cases" without examples.

## Output shape

- Goal
- Approach
- Change areas
- Step-by-step plan
- Verification plan
- Open questions or risks
