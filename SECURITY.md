# Security Policy

This repository is the **LAP runtime** behind the `central-repo-ops` control plane —
it runs locally at `localhost:4000` and handles provider keys. Treat it as **core**
tier (strictest handling). It is a private, single-maintainer fork; see
[`FORK.md`](FORK.md).

## Reporting a vulnerability
Report any suspected vulnerability or exposed secret **privately** to the maintainer
(@toasterman234). Do **not** open a public issue or PR containing exploit detail.
Rotate any leaked credential immediately — **purge ≠ revoke**.

## Supported versions
- The **deployed** version is pinned at tag `deployed-2026-06-20`. Only the pinned
  deployment and `main` are supported.
- Upstream security fixes are adopted deliberately per the update strategy in
  [`FORK.md`](FORK.md), not auto-tracked.

## Handling secrets
- No credentials belong in the repo. The gateway master key and provider keys are
  supplied via environment / `.env` at runtime and must never be committed.
- `sk-local` is the public upstream default dev key — not a secret by itself, but any
  real provider key is.
