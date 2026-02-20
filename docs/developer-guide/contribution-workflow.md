# Contribution Workflow

A disciplined path from idea to merged, reproducible change.

## 1. Intake and Scope

1. Understand the user/job outcome.
2. Confirm existing issue context (`bd ready`, `bd show <id>`).
3. Claim work (`bd update <id> --status in_progress`).

## 2. Implement in Small Increments

1. Make scoped changes.
2. Add or update tests with behavior changes.
3. Keep docs aligned in the same change set.

## 3. Validate Continuously

1. Run targeted tests during development.
2. Run `make check` before commit.
3. Run `make docs-check` for docs/style safety.
4. Run governance gates before push.

## 4. Close Work Properly

1. `bd close <id> --reason "implemented + verified"`
2. `bd sync`
3. `git add ...`
4. `git commit -m "type: summary"`
5. `git push`

## 5. Verify Landing

1. `git status` shows clean working tree.
2. Branch is up to date with remote.
3. Issue tracker reflects final status.

## Documentation Requirements

For notable changes, include:
1. What changed.
2. Why it changed.
3. How it was validated.
4. Where evidence lives.

## Anti-Patterns to Avoid

1. Large mixed-purpose commits.
2. Undocumented behavior changes.
3. Skipping gates because "it worked once locally".
4. Leaving local-only commits at session end.

In short: ship predictably, not poetically.
