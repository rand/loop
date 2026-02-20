# Diagnostics Checklist

Use this template for reproducible incident reports.
Unchecked boxes in this document are intentional; this is an operational form, not a backlog tracker.

Pair this checklist with `incident-playbook.md` for response sequencing.

## 1. Command Context

- Command executed:
- Working directory:
- Timestamp:
- Branch/commit:

## 2. Failure Snapshot

- Exit code:
- First failing step:
- Raw stderr/stdout artifact path:
- If command hung: elapsed wall time before termination:

## 3. Environment Snapshot

Run and capture:

```bash
rustc --version
python3 --version
go version
uv --version
git rev-parse HEAD
git status --short --branch
ps -axo pid=,command=,rss= -ww | rg -n "rlm_repl|lake env repl|\\brepl\\b" -S
```

## 4. Scope Classification

Mark one primary scope:
- [ ] Build/toolchain
- [ ] Test regression
- [ ] Governance/policy
- [ ] Runtime logic
- [ ] Integration compatibility
- [ ] Documentation mismatch

## 5. Minimal Repro Steps

Write shortest deterministic sequence that reproduces the issue.

## 6. Candidate Root Cause

State hypothesis in one paragraph and include relevant file references.

## 7. Fix and Verification

- Patch summary:
- Commands used to validate:
- Evidence artifact paths:

## 8. Closure Criteria

- [ ] Original failure resolved
- [ ] Full gate chain re-run
- [ ] No new regressions introduced
- [ ] Docs updated if behavior changed

This checklist is intentionally boring. Boring checklists prevent exciting incidents.
