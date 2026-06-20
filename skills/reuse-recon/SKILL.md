---
name: reuse-recon
description: Ben's search-before-building habit. Before building anything new, search durable memory and the filesystem for reusable parts and give a REUSE / EXTEND / BUILD-NEW verdict first. Use whenever a request is to build, create, set up, add, or implement something.
---

# Reuse-recon

Ben's single biggest time-sink is rebuilding things that already exist (the rewrite cycle). So before you write anything new, find what's already there and recommend reusing it.

## When this applies
Any request shaped like "build X", "create Y", "set up Z", "add a …", "implement …", or "we need a tool that …".

## The procedure (do this BEFORE writing code)
1. **Search durable memory** — `ben_memory_search` for the thing and its adjacent names (the capability, not just an exact title). Ben's systems are heavily interlinked; the part often exists under a different name.
2. **Search the filesystem / workspace** — look for existing implementations by capability (grep for the behavior, file search by purpose), not only the literal name.
3. **Classify each candidate found:**
   - **REUSE** — it already does this; use it as-is.
   - **EXTEND** — close; a small change to the existing thing covers it.
   - **BUILD-NEW** — nothing genuinely fits.
4. **Report the verdict first, with evidence** — name the files/paths you found and your recommendation, before building. Default to REUSE or EXTEND.

## If the verdict is BUILD-NEW
Say explicitly why the existing parts don't fit (not just "I didn't find one"). That justification is what protects Ben from another needless rewrite.

## Tone
Lead with the recommendation in one plain sentence ("Reuse the X at <path> — it already does this"), then the short evidence. Don't start coding until Ben has the verdict.
