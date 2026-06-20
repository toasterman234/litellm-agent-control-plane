# Why this fork exists

`toasterman234/litellm-agent-control-plane` is a **frozen, pinned fork** of
`LiteLLM-Labs/litellm-agent-control-plane`. It is the **LAP runtime** behind the
`central-repo-ops` control plane — the model brain at `localhost:4000`, $0-spend via
subscription. Tier: **core**.

## Decision of record
See **ADR-0001** in `central-repo-ops`
(`docs/decisions/0001-freeze-and-pin-lap-fork.md`). Summary:
- We **keep** the fork (it is genuinely customized) rather than delete + track upstream.
- Deployed code is **pinned** at tag `deployed-2026-06-20`.
- Local customizations live on branch `fix/skip-mcp-vault-for-non-anthropic`
  (backup commit `5ba5dbf`); `main` mirrors upstream.

## Update / pin strategy
- **Do not auto-track upstream into the deployment** — upstream `main` can overwrite
  our fixes. Pull upstream deliberately, re-test, then move the `deployed-*` tag.
- To adopt an upstream security fix: backport onto our branch, rebuild, verify at
  `:4000`, then re-tag `deployed-<date>`.

## Known follow-ups (tracked in central-repo-ops)
- **#20** — this core-tier baseline (governance files + enable Issues).
- Source currently builds from the **SSD**, against the no-runtime-on-SSD rule — move
  to internal disk (the SSD unmount on 2026-06-07 SIGBUS-crashed a live session).
- **#35** — model identity (runs report a model not in LAP's list).
