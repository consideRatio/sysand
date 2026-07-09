# Agent configuration layout

Single source of truth for repo-local agent instructions and skills, shared
across coding agents (Claude Code, OpenAI Codex, and any AGENTS.md-aware tool).
Everything here is gitignored — personal, repo-local setup.

Verified 2026-07-09 against Claude Code 2.1.205, Codex CLI 0.143.0, and live
official docs (code.claude.com/docs, developers.openai.com/codex, agents.md,
agentskills.io).

## Layout

| Path                             | Purpose                                                                                                                                                                                   |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `AGENTS.md` (repo root)          | Shared instructions. Read natively by Codex and ~30 other tools ([agents.md](https://agents.md/) standard, Linux Foundation).                                                             |
| `CLAUDE.md` (repo root)          | Bridge for Claude Code: `@AGENTS.md` import (Claude Code does **not** read AGENTS.md natively — confirmed against Anthropic docs 2026-07). Claude-only overrides may go below the import. |
| `.agents/skills/<name>/SKILL.md` | Shared skills, one directory per skill ([Agent Skills spec](https://agentskills.io/specification)).                                                                                       |
| `.claude/skills`                 | Symlink `-> ../.agents/skills`. Claude Code does not read `.agents/skills` natively; it does follow symlinks (officially supported).                                                      |

No `.codex/` directory is needed: Codex discovers `.agents/skills` natively
(its documented project location; verified empirically by removing the symlink
and running `codex exec` — project skills were still listed).

If a Claude-only skill (using non-portable frontmatter like `context: fork`,
`hooks`, `paths`) is ever needed, switch from the whole-dir symlink to
per-skill symlinks (`.claude/skills/<name> -> ../../.agents/skills/<name>`)
and keep the Claude-only skill as a real directory under `.claude/skills/`.

Windows caveat (not relevant on this machine): git checks symlinks out as
plain text files under the default `core.symlinks=false`; use per-tool copies
or the `@AGENTS.md`-style import instead there.

## Verifying discovery

```sh
codex exec --skip-git-repo-check -s read-only \
  "Do not use tools. List the names of your available skills."
claude -p "Do not use tools. List the names of your available skills." --model haiku
```

Both should list the skills under `.agents/skills/`.

## Writing skills (portable core)

Create `.agents/skills/<name>/SKILL.md`:

```markdown
---
name: <name> # must match the directory name; lowercase/digits/hyphens
description: Use when ... (trigger conditions — both tools match on this)
---
```

- Only `name` and `description` are portable. Claude extensions
  (`argument-hint`, `allowed-tools`, `model`, ...) are ignored harmlessly by
  Codex; Codex-only invocation policy goes in `agents/openai.yaml` inside the
  skill, invisible to Claude.
- Keep SKILL.md under ~500 lines / 5k tokens. Push bulk into `references/`
  and `scripts/` subdirs, loaded on demand with explicit triggers in the body
  ("read references/x.md if ...") — progressive disclosure, one level deep.
- Highest-value content: gotchas (environment facts that defy assumptions),
  checklists for multi-step workflows, output templates, and validation loops
  (do → run check → fix → repeat). Test: "would the agent get this wrong
  without the instruction?" If no, cut it.

## What goes where

- **AGENTS.md** — facts needed every session: build commands, conventions,
  layout, "always do X". Keep under ~200 lines; longer files reduce adherence.
- **Skills** — anything that has grown into a _procedure_ rather than a fact.
- **Hooks / permission settings** — anything that must _always_ happen
  (markdown is guidance, not enforcement).
- **Nested AGENTS.md** (subdirectories) — path-specific guidance; nearest file
  wins for Codex, Claude Code concatenates ancestors and lazy-loads
  subdirectory CLAUDE.md files.

## Self-improvement loop

When an agent makes a mistake that gets corrected — especially a second time —
record the lesson immediately:

- procedure-shaped → add/extend a skill under `.agents/skills/` (usually its
  gotchas section);
- fact-shaped → one line in `AGENTS.md`.

This is the shared-learning channel; Claude Code's auto memory is machine-local
and complements it but doesn't replace it.
