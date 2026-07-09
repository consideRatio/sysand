---
name: plan-review
description: Plan a task, have it scrutinized by a staff engineer reviewer, iterate once, then present the final plan. Use when the user wants a reviewed/vetted implementation plan.
argument-hint: [task description]
---

## Reviewed Plan Workflow

You are executing a structured plan-review workflow for the following task:

$ARGUMENTS

### Step 1: Draft the Plan

Use the current agent's planning mechanism. Explore the codebase as needed, then draft a detailed implementation plan in the conversation. Do not write plans to `tasks/`. If the user explicitly asks for a saved plan artifact and does not specify a path, write it under `.agents/plans/` with a descriptive filename. The plan should include:

- Clear problem statement
- Proposed approach with rationale
- Files to create/modify
- Step-by-step implementation checklist
- Edge cases and risks considered

### Step 2: Staff Engineer Review

Once the plan is drafted, get a **senior staff software engineer reviewer** pass. If subagents are available and the user explicitly asked for delegated/subagent review, spawn an agent with the drafted plan in the prompt or, if the user explicitly requested a saved plan artifact, with that file path. Use a prompt like:

> You are a staff software engineer reviewing an implementation plan. Review the plan below and the relevant codebase context. Critique the plan on:
>
> 1. **Correctness**: Will this actually solve the problem? Any logical gaps?
> 2. **Simplicity**: Is this the simplest approach? Any over-engineering?
> 3. **Risk**: What could go wrong? Missing edge cases? Migration risks?
> 4. **Maintainability**: Will this be easy to understand and modify later?
> 5. **Completeness**: Are there missing steps, tests, or documentation updates?
> 6. **Alternatives**: Is there a better approach the author didn't consider?
>
> Be direct and specific. Flag concrete issues, not vague concerns. Suggest specific improvements where possible. Read `AGENTS.md` or `CLAUDE.md` for project conventions, depending on the current agent. Do not edit files unless explicitly instructed.

If subagents are unavailable or not explicitly requested, perform the staff-review pass yourself and clearly label it as a self-review.

### Step 3: Iterate on the Plan

Review the staff engineer's feedback. Revise the plan in the conversation to address valid concerns. If you disagree with a point, note your reasoning. If the user explicitly requested a saved plan artifact, update the corresponding plan file instead of writing to `tasks/`.

### Step 4: Present to User

Present the final plan to the user with:

- A concise summary of the approach
- Key changes made after the staff review
- Any open questions or trade-offs that need the user's input
