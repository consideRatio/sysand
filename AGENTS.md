## Workflow Orchestration

### 1. Plan Mode Default

- Enter plan mode for ANY non-trivial task (3+ steps or architectural decisions)
- If something goes sideways, STOP and re-plan immediately - don't keep pushing
- Use plan mode for verification steps, not just building
- Write detailed specs upfront to reduce ambiguity

### 2. Subagent Strategy to keep main context window clean

- Offload research, exploration, and parallel analysis to subagents
- For complex problems, throw more compute at it via subagents
- One task per subagent for focused execution

### 3. Verification Before Done

- Never mark a task complete without proving it works
- Diff behavior between main and your changes when relevant
- Ask yourself: "Would a staff engineer approve this?"
- Run tests, check logs, demonstrate correctness

### 4. Demand Elegance (Balanced)

- For non-trivial changes: pause and ask "is there a more elegant way?"
- If a fix feels hacky: "Knowing everything I know now, implement the elegant solution"
- Skip this for simple, obvious fixes - don't over-engineer
- Challenge your own work before presenting it

### 5. Autonomous Bug Fixing

- When given a bug report: just fix it. Don't ask for hand-holding
- Point at logs, errors, failing tests -> then resolve them
- Zero context switching required from the user
- Go fix failing CI tests without being told how

## Task Management

1. **Plan First**: Write plan with checkable items
2. **Verify Plan**: Check in before starting implementation
3. **Explain Changes**: High-level summary at each step

## Core Principles

- **Simplicity First**: Make every change as simple as possible. Impact minimal code.
- **No Laziness**: Find root causes. No temporary fixes. Senior developer standards.
- **Minimal Impact**: Changes should only touch what's necessary. Avoid introducing bugs.

## General Principles

When I ask you to implement something, do NOT add extra features, tables, or
functionality I didn't request. Stick strictly to what I asked for. If you think
something else is needed, ask first.

## Repo-Local Skills

Develop skills intended for use by Claude and/or Codex under `.agents/skills`.
Do not put repo-specific skills under `.codex/skills`, `.claude/skills`,
`~/.codex/skills`, or `~/.claude/skills`. Codex reads `.agents/skills`
natively; Claude Code reads it via the `.claude/skills` symlink. See
`.agents/README.md` for the full layout, portable SKILL.md frontmatter, and
verification commands.

## Recording Lessons

When you make a mistake that gets corrected — especially the same mistake a
second time — record the lesson before moving on: procedure-shaped lessons go
into the relevant skill under `.agents/skills` (gotchas section);
fact-shaped lessons become one line in this file. Keep this file under
200 lines.

## Communication Style

When I ask a conceptual or knowledge question, start with a direct answer before
diving into code or investigation. Don't immediately start writing code or
running tools unless I've asked for implementation.
