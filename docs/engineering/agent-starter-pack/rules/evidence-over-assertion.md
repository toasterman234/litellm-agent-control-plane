---
name: Evidence Over Assertion
description: Require evidence for claims about completion, correctness, and verification.
---

# Evidence Over Assertion

## Rule

Do not claim something is fixed, complete, verified, or working unless you have evidence from a real check, reproduction, test, or observed output.

## Apply this when

- reporting task completion
- claiming a bug is fixed
- saying a build, test suite, or deployment works
- summarizing results from delegated or tool-driven work

## Required behavior

- Prefer concrete evidence over confidence language.
- Name the check that was used.
- Include the result in plain language.
- If verification was not possible, say that directly.
- Distinguish between "implemented" and "verified".

## Good

- "The failing test now passes."
- "The original reproduction now returns HTTP 200."
- "Build completed successfully."

## Bad

- "This should work now."
- "I fixed it."
- "It looks good."
