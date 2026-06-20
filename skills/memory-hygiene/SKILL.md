---
name: memory-hygiene
description: When and how to use Ben's shared durable memory well — recall before assuming, save one self-contained fact at a time, never save secrets or chatter. Use whenever deciding whether to call ben_memory_search or ben_memory_save.
---

# Memory hygiene

You share a durable memory (the `ben_memory_search` / `ben_memory_save` tools) with Ben's other agents. Treat it as a shared brain: what you write, future sessions and other agents read. Keep it clean and useful.

## When to RECALL (`ben_memory_search`)
Call it BEFORE answering whenever the request touches:
- prior decisions, preferences, or "what we said earlier"
- a named project, system, or ongoing build
- anything where assuming wrong would waste Ben's time

Search first, then answer. If you used recalled memory, say so briefly. If nothing relevant came back, say what you couldn't recover instead of guessing.

## When to SAVE (`ben_memory_save`)
Save when a durable thing is established that a future session should know:
- a decision and its reason
- a preference or constraint Ben stated
- a non-obvious fact or an outcome that was verified

Rules for a good memory:
- **One self-contained fact per save.** It must stand alone — name the subject, don't write "it" or "this".
- Add 1–3 short tags to aid recall.
- **Search first** to avoid duplicates; if something similar exists, prefer updating intent over piling on a near-duplicate.

## What NOT to save
- Secrets, tokens, API keys, passwords — never.
- Transient conversation context or chatter ("ok", "running", step-by-step noise).
- Things already recorded in the codebase, git history, or a repo's docs.
- Test/scratch entries — they pollute the shared brain for every other agent.

## Default posture
When unsure whether a fact is worth saving, ask: "Would a future agent be wrong or slower without this?" If yes, save it cleanly. If no, skip it.
