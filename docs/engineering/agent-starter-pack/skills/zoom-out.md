---
name: Zoom Out
description: Step back and explain the broader system context, architecture, or module map before going deeper.
---

# Zoom Out

Use this skill when local details are not enough and a higher-level map is needed first.

## Use this skill when

- the code area is unfamiliar
- the user asks for architecture context
- the change spans multiple modules
- local fixes keep failing because the broader flow is unclear

## Workflow

1. Identify the user-facing job of the system or feature.
2. Map the main modules, boundaries, and data flow.
3. Explain which parts matter for the current task.
4. Highlight the likely leverage points before diving back in.

## Required behavior

- Use the project's own terminology where possible.
- Focus on relationships, not file dumps.
- Help the user understand where the current task fits.

## Output shape

- What this part of the system is for
- Main components
- How data or control flows through them
- Where the current task likely belongs
