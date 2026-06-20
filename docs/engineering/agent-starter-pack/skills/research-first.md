---
name: Research First
description: Search for existing tools, libraries, services, patterns, or prior art before building something new or making a recommendation.
---

# Research First

Before proposing a new build, implementation, or recommendation, first check whether a strong existing option already exists.

## Use this skill when

- the user wants to build or add something non-trivial
- the user asks for tool or library recommendations
- you are about to implement functionality from scratch
- you need to compare build-vs-buy-vs-adopt options

## Workflow

1. Restate the problem and key constraints.
2. Check the current project context first.
3. Search for existing tools, libraries, templates, services, or prior art.
4. Compare the best candidates for fit, maintenance, and risk.
5. Recommend one option and explain why it fits.
6. Only then proceed with implementation or setup.

## Output shape

- Recommended option
- Why it fits
- What custom work remains
- One or two alternatives with tradeoffs

## Guardrails

- Do not fabricate candidates.
- Do not skip research for medium or large build requests.
- If nothing suitable exists, say that clearly and proceed.
