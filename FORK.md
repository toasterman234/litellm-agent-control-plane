# Why this fork exists

`toasterman234/litellm-agent-control-plane` is a **frozen, pinned fork** of
`LiteLLM-Labs/litellm-agent-control-plane`. It is the **LAP agent platform** (Rust
control plane with agents door, rules engine, MCP vault) — deployed on **ZimaOS at
`:4002`**, not the gh-aw model brain. Tier: **core**.

**Model inference for gh-aw workflows** runs through the separate **LiteLLM Python
gateway** on ZimaOS (`http://192.168.1.121:14000/v1`, failover) — see ADR-0005 in
`central-repo-ops`. Both gateways route to cliproxy on the Mac → $0 real spend.

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
  `:4002` on zima, then re-tag `deployed-<date>`.

## Known follow-ups (tracked in central-repo-ops)
- **#20** — this core-tier baseline (governance files + enable Issues).
- **#36** — **closed 2026-06-25** — LAP migrated off Mac SSD to ZimaOS `:4002`; SSD
  path is cold storage only.
- **#35** — model identity (runs report a model not in LAP's list).
