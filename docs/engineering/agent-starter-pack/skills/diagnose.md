---
name: Diagnose
description: Use a disciplined diagnosis loop for bugs, regressions, and failures before jumping into fixes.
---

# Diagnose

Use a structured debugging process before making fixes.

## Use this skill when

- a bug is reported
- a test fails
- a build breaks
- behavior is unexpected
- a performance regression is suspected

## Workflow

1. Build a reliable feedback loop.
2. Reproduce the failure clearly.
3. Generate multiple ranked hypotheses.
4. Test one hypothesis at a time.
5. Fix the confirmed root cause.
6. Re-run the original reproduction and any regression check.

## Required behavior

- Do not jump straight to patching.
- Prefer real reproductions over guesswork.
- Separate symptoms from root cause.
- Explain the winning hypothesis and why it matched the evidence.

## Output shape

- Symptom
- Reproduction or feedback loop
- Ranked hypotheses
- Confirmed root cause
- Fix
- Verification result
