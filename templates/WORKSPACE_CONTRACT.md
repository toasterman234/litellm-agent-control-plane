# Workspace Contract

Every LAP runtime template must implement the same workspace discovery rules.

## Purpose

Agents must not guess where workspace rules or skills live. They must use the
same contract across current and future runtimes so new rules or skills become
visible without runtime-specific prompt surgery.

## Workspace Root Resolution

Runtimes should resolve the workspace root in this order:

1. Session environment config:
   - `workspace_path`
   - `workspacePath`
   - `workdir`
   - `cwd`
   - `root_dir`
   - `rootDir`
   - `path`
   - `workspace.path`
   - `workspace.root`
   - `workspace.root_dir`
   - `source.path`
   - `source.root`
   - `source.root_dir`
2. Process env fallback:
   - `LAP_DEFAULT_WORKSPACE`
3. Runtime-local scratch workspace

Runtimes should prefer a real mounted workspace over an empty scratch folder.

## Authoritative Rules

Treat these as workspace rules, in order of locality:

1. Nearest `AGENTS.md`
2. Nearest `CLAUDE.md`
3. Repo-wide `CODING_STANDARDS.md`
4. `.agent/rules/**/*.md`

If scoped instruction files exist deeper in the tree, they override broader
ones for work inside that subtree.

## Skills

Treat these as skills, not rules:

1. `.agent/skills/**/*.md`
2. `skills/**/*.md`

Skills are procedures or reusable capabilities. They are not governing rules
unless a rule file explicitly says they are mandatory.

## Runtime Behavior

Every runtime template should:

1. Mount a default workspace into the container for local development.
2. Expose or honor `LAP_DEFAULT_WORKSPACE`.
3. Tell the agent where the authoritative workspace root is.
4. Explicitly distinguish rules from skills in runtime instructions.
5. Avoid writing generated instruction files back into the user's repo unless
   the user asked for that.

## Scaffolding Requirement

New runtime templates must adopt this contract during scaffolding. Do not add a
new template that invents a different workspace discovery scheme.
