---
name: commit
description: Use when the user asks to create, split, review, polish, or prepare git commits in this repository.
argument-hint: "[commit request]"
---

# Commit Workflow

Create small, coherent commits that match the user's requested scope.

For message-only requests:

- If the user asks only to draft, simulate, polish, or review a commit message,
  do not run the full commit workflow.
- Prefer the supplied diff, staged diff, or stated change summary as the source
  of truth.
- Skip verification commands unless the user asks whether the change is ready to
  commit.
- Return the commit message directly; include analysis only when the user asks
  for alternatives or critique.
- Use a subject-only message when the change is obvious from the subject. Add a
  body only to explain non-obvious behavior, motivation, or risk.

Before committing:

- Inspect `git status --short`.
- Review the staged and unstaged diff for every file that will be committed.
- Do not include unrelated user changes.
- Run the relevant verification for the touched files; for this repo, run
  `uv run prek run --all-files` before finishing.
- Treat exit code 0 from `uv run prek run --all-files` as sufficient evidence
  that formatting and linting passed; do not run an extra post-check diff just
  to see whether formatter hooks changed files.
- If that verification has already passed, create the commit with
  `git commit --no-verify` to avoid running the same hooks again.

When writing commit messages:

- Use a concrete imperative subject, such as `Add repo-local commit skill`.
- Mention the functional reason in the body when the change is not obvious.
- Wrap body text at roughly 72 columns.
- Avoid generic messages like `fix stuff`, `updates`, or `wip`.

If the working tree contains unrelated changes, stage only the intended paths
and call out any remaining uncommitted work in the final response.
