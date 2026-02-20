# Workflow Recipes

These are opinionated runbooks for common jobs-to-be-done.

## Recipe 1: "I need a fast confidence check"

```bash
make check
./scripts/dp review --json
```

Success criteria:
- `make check` passes
- `review` returns `"ok": true`

Use when:
- You changed code and want local confidence before deeper validation.

## Recipe 2: "I need release-grade confidence"

```bash
make check
./scripts/dp verify --json
./scripts/dp enforce pre-commit --policy dp-policy.json --json
./scripts/dp enforce pre-push --policy dp-policy.json --json
```

Success criteria:
- All commands return exit `0`
- JSON outputs include `"ok": true`

Use when:
- You are about to merge or push important changes.

## Recipe 3: "I need to formalize NL requirements"

1. Capture concise requirements language.
2. Run spec-agent workflow in your integration surface.
3. Confirm generated artifacts include required level:
- Types
- Invariants
- Contracts
- Full proofs (when needed)
4. Run verification gates.

Read more:
- `docs/spec/SPEC-20-typed-signatures.md`
- `docs/spec/SPEC-22-proof-protocol.md`
- `docs/spec/SPEC-27-fallback-extraction.md`

## Recipe 4: "A gate failed and I need triage"

1. Re-run the exact failing command.
2. Capture full output (no screenshots of terminal corners).
3. Classify failure:
- Build/toolchain
- Test regression
- Governance policy failure
- Environment issue
4. Apply targeted fix.
5. Re-run complete gate sequence.

Then use:
- [Troubleshooting Common Issues](../troubleshooting/common-issues.md)
- [Diagnostics Checklist](../troubleshooting/diagnostics-checklist.md)

## Recipe 5: "I need to hand off cleanly"

Before handoff:
1. `git status`
2. `make check`
3. `./scripts/dp verify --json`
4. Commit with clear scope.
5. Push.
6. Ensure branch is up to date with remote.

Because nothing says "teamwork" like not leaving broken local-only miracles.

## Recipe 6: "I need confidence in Claude adapter behavior"

```bash
make claude-adapter-gate
```

Success criteria:
- `VG-CLAUDE-ADAPTER-E2E-001` passes
- At least two scenario tests execute (guardrail against zero-test filter drift)

Use when:
- You changed `adapters/claude_code/*`
- You changed complexity signals, context externalization, or adapter mode logic
- You want evidence for observe/orient/decide/act behavior, not just unit plumbing

Read:
- `/Users/rand/src/loop/docs/user-guide/claude-code-adapter.md`
