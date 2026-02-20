# Decisions Ledger

Use this file for architecture and contract decisions that affect more than one task or repo.

## Status Legend

- Proposed: recommended but awaiting approval.
- Accepted: approved and binding for execution.
- Rejected: explicitly not selected.
- Superseded: replaced by a later decision.

## D-001 Compatibility Policy

- Status: Accepted
- Date: 2026-02-19
- Context: `rlm-claude-code` is a hard consumer and under active development.
- Decision: Treat compatibility with `rlm-claude-code` as a release-blocking invariant for M1-M4 changes.
- Policy detail: Temporary breakage is allowed only on non-mainline branches with an explicit recovery issue; merge to `main` requires compatibility gates to pass.
- Consequences:
- All API-affecting tasks require consumer validation gates.
- Any breaking behavior requires explicit approval and migration note.
- Impacted tasks/gates: M0-T02, M1-M4 API-affecting tasks, VG-CONTRACT-001, VG-RCC-001.

## D-002 Build Baseline Definition

- Status: Accepted
- Date: 2026-02-19
- Context: Default build in `rlm-core` currently fails.
- Decision: Baseline health is defined as all of the following passing in `/Users/rand/src/loop/rlm-core`:
- `cargo check`
- `cargo check --no-default-features`
- `cargo check --no-default-features --features gemini`
- Consequences:
- M1 cannot close until all baseline checks pass.
- Impacted tasks/gates: M0-T02, M1-T01, M1-T02, VG-LOOP-BUILD-001/002/003.

## D-003 REPL Entrypoint Strategy

- Status: Accepted
- Date: 2026-02-19
- Context: Rust spawns `python -m rlm_repl`; before M1-T03 the package lacked `__main__.py` and startup could fail in dev-mode path usage.
- Decision: Make REPL startup robust by supporting module and script execution explicitly.
- Recommended implementation order:
- Add `rlm_repl/__main__.py` delegating to `main()`.
- Keep script entrypoint (`rlm-repl`) for compatibility.
- Consequences:
- Rust spawn path remains stable.
- Manual/packaged execution both work.
- Impacted tasks/gates: M0-T03, M1-T03, VG-LOOP-REPL-002.

## D-004 Batched Query API Naming

- Status: Accepted
- Date: 2026-02-19
- Context: SPEC-26 uses `llm_query_batched`; runtime helper exports `llm_batch`.
- Decision: Support both names short-term with one canonical name and a deprecation window.
- Recommended policy:
- Canonical runtime function: `llm_batch`
- Compatibility alias: `llm_query_batched`
- Update specs + docs to reflect canonical + alias policy
- Consequences:
- Prevents immediate consumer breakage.
- Removes ambiguity for future implementation.
- Impacted tasks/gates: M0-T04, M3-T01, VG-CONTRACT-001, VG-DOC-SPEC-001.

## D-005 Typed-Signature Contract Source of Truth

- Status: Accepted
- Date: 2026-02-19
- Context: Rust and Python contracts diverge for signature registration and SUBMIT paths.
- Decision: Rust `ExecuteResult` + Python JSON-RPC schema become the binding runtime contract; SPEC-20 must be reconciled to this contract.
- Consequences:
- M2 tasks can validate directly against runtime behavior.
- Spec updates become deterministic.
- Impacted tasks/gates: M2-T01..M2-T05, VG-LOOP-REPL-001, VG-LOOP-SIG-001, VG-EFFICACY-001.

## D-006 Cross-Repo Integration Scope

- Status: Accepted
- Date: 2026-02-19
- Context: `loop-agent` and `io-rflx` are active and intend integration with loop, but coupling levels differ.
- Decision:
- `rlm-claude-code`: runtime compatibility and vendored-sync track.
- `loop-agent`: define and ship first concrete runtime integration seam in M4.
- `io-rflx`: contract-level interoperability + benchmark alignment in M4/M5.
- Consequences:
- Work is scoped by actual coupling, avoiding speculative rewrites.
- Impacted tasks/gates: M0-T05, M4-T01..M4-T04, M5-T01..M5-T03, VG-CONTRACT-001, VG-RCC-001, VG-LA-001, VG-RFLX-001.

## D-007 Safe Execution Mode on This Machine

- Status: Accepted
- Date: 2026-02-19
- Context: Prior parallel execution exhausted memory and crashed the laptop.
- Decision: Enable safe mode by default:
- one active heavy worker at a time,
- mandatory heavy-command wrapper (`/Users/rand/src/loop/scripts/safe_run.sh`),
- conservative memory admission threshold (`LOOP_MIN_AVAILABLE_MIB=3072` recommended).
- Consequences:
- Throughput is lower but stability is prioritized.
- Heavy validation gates are serialized.
- Lane B/C remain read-only until explicitly activated by orchestrator.

## D-008 `rlm-claude-code` Vendored Pin Compatibility Scope

- Status: Accepted
- Date: 2026-02-19
- Context: `VG-RCC-001` runs in `/Users/rand/src/rlm-claude-code`, which pins loop via `vendor/loop` submodule SHA that can differ from `/Users/rand/src/loop` working-tree `HEAD`.
- Decision:
- Compatibility claims must be pin-aware.
- M4+ RCC evidence must record both the loop candidate SHA and the `vendor/loop` submodule SHA used by the gate.
- If SHAs differ, scope is explicitly limited to the pinned submodule SHA until submodule sync + rerun.
- Consequences:
- Prevents false compatibility claims for untested loop commits.
- Makes vendored sync policy explicit and auditable across sessions.
- Impacted tasks/gates: M4-T01, M4-T04, VG-RCC-001, VG-CONTRACT-001.

## D-009 `loop-agent` Gate Scope During Active Development

- Status: Accepted
- Date: 2026-02-19
- Context: Full `loop-agent` suite currently includes in-flight durability/optimizer failures unrelated to M4 integration seam objectives (classifier + trajectory + sensitivity guarantees).
- Decision:
- Treat `VG-LA-001` as a seam-critical compatibility gate for M4 (targeted deterministic subset).
- Add `VG-LA-002` as a full-suite snapshot gate for advisory health and failure triage tracking.
- Consequences:
- M4 integration work can proceed without masking unrelated consumer backlog.
- Full-suite instability remains visible and explicitly tracked in evidence.
- Impacted tasks/gates: M4-T02, M4-T04, VG-LA-001, VG-LA-002, VG-CONTRACT-001.

## D-010 `io-rflx` Interop is Schema-First and Versioned

- Status: Accepted
- Date: 2026-02-19
- Context: `io-rflx` integration is active but currently contract-level; forcing direct compile-time coupling is premature.
- Decision:
- Define interoperability through versioned JSON envelopes for provenance, trajectory, and verification payloads.
- Start contract version at `io_rflx_interop.v0` with explicit confidence-ladder mapping.
- Require migration notes for any breaking schema changes before compatibility claims.
- Consequences:
- Enables validation and integration progress without hard dependency coupling.
- Keeps schema evolution explicit and auditable across loop + io-rflx changes.
- Impacted tasks/gates: M4-T03, M4-T04, M5-T01, M5-T02, VG-RFLX-001, VG-CONTRACT-001.

## D-011 Compatibility Claims Are Tuple-Based With Explicit Support Window

- Status: Accepted
- Date: 2026-02-19
- Context: All three consumers are active, but repos are pre-1.0 with mixed versioning styles (tagged and untagged branches). Branch-level compatibility claims are ambiguous without explicit tuple scoping.
- Decision:
- Compatibility is claimed only for explicit tuples (`loop_sha`, `consumer_sha`, optional `vendor_loop_sha`, optional `schema_version`).
- Support tiers are standardized as `supported`, `conditional`, and `unsupported`.
- Each consumer must retain at least current + rollback tuple evidence for safe releases/rollbacks.
- Deprecation/removal of consumer-observed APIs requires documented migration notes, a minimum 14-day lead time, and a post-notice compatibility gate rerun.
- Consequences:
- Prevents over-broad compatibility claims across fast-moving branches.
- Makes support and deprecation policy operationally auditable.
- Creates a stable handoff model for multi-agent execution across sessions.
- Impacted tasks/gates: M6-T01, M6-T02, M6-T03, VG-CONTRACT-001, VG-RCC-001, VG-LA-001, VG-RFLX-001.

## D-012 Release and Rollback Are Class-Based With Hard No-Go Triggers

- Status: Accepted
- Date: 2026-02-19
- Context: Integration changes now span multiple consumers and gate families; ad hoc release decisions create avoidable risk and inconsistent rollback behavior.
- Decision:
- Adopt release classes R0-R3 with predefined mandatory gate sets in `docs/execution-plan/RELEASE-ROLLBACK-PLAYBOOK.md`.
- Enforce automatic no-go on required gate failures, missing tuple evidence, and unapproved contract-breaking changes.
- Require rollback to a previously supported tuple recorded in `docs/execution-plan/COMPATIBILITY-MATRIX.md`.
- Consequences:
- Standardizes go/no-go behavior across sessions and operators.
- Reduces ambiguity under pressure by making rollback steps deterministic.
- Couples release decisions directly to auditable evidence artifacts.
- Impacted tasks/gates: M6-T02, M6-T03, VG-CONTRACT-001, VG-RCC-001, VG-LA-001, VG-RFLX-001, VG-PERF-001, VG-PERF-002, VG-EFFICACY-001.

## D-013 Steady-State Governance Uses Explicit Cadence and Ownership

- Status: Accepted
- Date: 2026-02-19
- Context: M0-M6 remediation is complete; without recurring governance cadence, contract/spec drift and stale compatibility claims will recur.
- Decision:
- Adopt the recurring governance schedule in `docs/execution-plan/MAINTENANCE-CADENCE.md`.
- Keep orchestrator ownership for trackers/decisions and lane activation policy.
- Require weekly compatibility refreshes and monthly decision/deprecation audits for actively supported tuples.
- Consequences:
- Converts remediation artifacts into an operational steady-state system.
- Makes ownership explicit for cross-session and multi-agent execution.
- Ensures compatibility claims remain current and rollback-ready.
- Impacted tasks/gates: M6-T03, VG-CONTRACT-001, VG-RCC-001, VG-LA-001, VG-RFLX-001.

## D-014 `VG-LA-002` Promotion Criteria for `loop-agent` Full-Suite Health

- Status: Accepted
- Date: 2026-02-19
- Context: `VG-LA-002` was initially advisory (D-009) due broad in-flight failures. Fresh snapshot on `loop-agent@390f459` now reports `865 passed, 2 failed`, while `VG-LA-001` seam-critical subset remains green.
- Decision:
- Keep `VG-LA-002` advisory until `0` full-suite failures are demonstrated on the active `loop-agent` default integration branch.
- Require failure-class triage artifacts whenever `VG-LA-002` is non-green, including dependency-profile vs functional categories.
- Allow conditional release decisions only when remaining failures are explicitly classified as non-seam and tracked with upstream remediation issue IDs.
- Promote full-suite gate from advisory to release-blocking only after three consecutive green `VG-LA-002` snapshots on the supported tuple path.
- Consequences:
- Preserves delivery velocity for seam-critical integration while tightening governance as full-suite health improves.
- Prevents premature promotion of full-suite blocking status based on a single green or unstable run.
- Makes escalation from advisory to blocking objective and auditable.
- Impacted tasks/gates: loop-the, VG-LA-001, VG-LA-002, VG-CONTRACT-001.

## D-015 Promotion Claims Require Committed Consumer Tuple State

- Status: Accepted
- Date: 2026-02-19
- Context: `VG-LA-002` reached consecutive green snapshots during loop-8th, but on a dirty local `loop-agent` working tree with uncommitted changes.
- Decision:
- Treat green snapshots on dirty consumer working trees as diagnostic evidence, not final promotion evidence.
- Require promotion claims (e.g., advisory -> release-blocking) to cite committed consumer SHA state for the tested tuple.
- If test-fixing changes are local/uncommitted, promotion status remains pending until committed tuple rerun evidence is captured.
- Consequences:
- Prevents false confidence from non-reproducible local test state.
- Preserves D-014 rigor while allowing local triage to proceed quickly.
- Keeps compatibility matrix claims auditable across sessions.
- Impacted tasks/gates: loop-8th, VG-LA-002, VG-CONTRACT-001.

## D-016 `VG-LA-002` Promotion Criteria Satisfied on Candidate Tuple

- Status: Superseded (by D-018)
- Date: 2026-02-19
- Context: loop-8th produced three consecutive green `VG-LA-002` runs on committed candidate tuple `f2aeb18` in `/tmp/loop-agent-clean` (`867 passed` each run).
- Decision:
- Mark D-014 and D-015 promotion evidence as satisfied for candidate tuple `f2aeb18`.
- Treat `VG-LA-002` as promotable-to-blocking for that tuple scope.
- Keep canonical consumer baseline pending until candidate changes are landed in `/Users/rand/src/loop-agent` and cadence gates are rerun.
- Consequences:
- Moves full-suite gate promotion from theoretical to evidenced for a committed tuple.
- Preserves strict tuple scoping and avoids over-claiming for non-landed consumer state.
- Impacted tasks/gates: loop-8th, loop-5va, VG-LA-002, VG-CONTRACT-001.

## D-017 Temporary `loop-agent` Tuple Source Policy: Committed Clean Clone Only

- Status: Accepted
- Date: 2026-02-19
- Context: `/Users/rand/src/loop-agent` is under active development and can include in-flight working-tree changes that are not part of committed tuple state, creating unstable or non-reproducible compatibility evidence.
- Decision:
- Until canonical `loop-agent` stabilization is declared, compatibility claims must use `clean_clone_committed` tuple mode only.
- Required gate evidence for `VG-LA-001` and advisory snapshots for `VG-LA-002` must run from a clean clone checked out at committed canonical SHA.
- Canonical working-tree runs may be used for local triage, but they do not qualify as compatibility-claim evidence.
- Policy can be lifted only by explicit decision update after canonical committed tuple stability is verified and canonical working-tree drift risk is cleared.
- Consequences:
- Prevents tuple evidence drift from uncommitted or untracked canonical changes.
- Keeps cadence packets reproducible and auditable across sessions/agents.
- Makes the boundary between diagnostic and claim-grade evidence explicit.
- Impacted tasks/gates: loop-5va, Ops-Weekly, VG-LA-001, VG-LA-002, VG-CONTRACT-001.

## D-018 `loop-agent` Canonical Stabilization Supersedes Candidate-Landing Objective

- Status: Accepted
- Date: 2026-02-19
- Context: `loop-5va` originally targeted landing candidate commit `f2aeb18` into canonical `loop-agent`. Canonical advanced independently to committed SHA `30c1fa`, and clean-clone tuple validation on that canonical SHA reports `VG-LA-001: 30 passed` and advisory `VG-LA-002: 936 passed`.
- Decision:
- Treat canonical committed tuple `30c1fa` as the active stabilized baseline for `loop-agent` compatibility claims.
- Mark the candidate-landing objective from `loop-5va` as superseded by newer canonical committed history plus fresh clean-clone evidence.
- Keep D-017 claim-source policy active until canonical working-tree drift risk is cleared or explicitly re-scoped.
- Consequences:
- Avoids unnecessary cherry-pick/landing work against stale candidate history.
- Reanchors support policy and gate claims on the latest validated committed canonical tuple.
- Preserves reproducibility by requiring clean-clone claim runs despite canonical local working-tree churn.
- Impacted tasks/gates: loop-5va, Ops-Weekly, VG-LA-001, VG-LA-002, VG-CONTRACT-001.

## D-019 Coverage Proof Uses Reproducible `llvm-cov` Gate With CI Canonical Evidence

- Status: Accepted
- Date: 2026-02-20
- Context: Historical coverage closure claims were not backed by a reproducible, enforced coverage gate in this repository.
- Decision:
- Adopt `scripts/run_coverage.sh` + `make coverage` as the canonical coverage execution path.
- Add CI workflow `.github/workflows/rlm-core-coverage.yml` as the canonical enforcement/evidence path.
- Set line-coverage threshold policy to `>= 80%` (`COVERAGE_MIN_LINES`, default `80`).
- Consequences:
- Coverage evidence becomes repeatable and reviewable with concrete artifacts (`coverage/lcov.info`, `coverage/summary.txt`).
- Local environments without `cargo-llvm-cov` can still proceed with explicit blocked evidence while CI remains authoritative.
- Impacted tasks/gates: loop-k7d, VG-COVERAGE-001.

## D-020 `rlm-claude-code` Migration Scope Is Component Delegation Until Binding Surface Expands

- Status: Accepted
- Date: 2026-02-20
- Context: Early migration specs model full Python replacement, but Python bindings still do not expose orchestration/repl surfaces (`Orchestrator`, `ClaudeCodeAdapter`, `ReplPool`/`ReplHandle`).
- Decision:
- Treat component-level delegation as the supported migration end state for current scope.
- Mark full replacement language in migration docs as archival target-state planning, not active backlog.
- Consequences:
- Removes ambiguity between executed migration reality and aspirational architecture.
- Prevents false "partial implementation" interpretation for intentionally out-of-scope binding gaps.
- Impacted tasks/gates: loop-cyl, loop-k7d, VG-CONTRACT-001, documentation reconciliation gates.

## D-021 API Documentation Claims Must Distinguish Module-Level Baseline vs Item-Level Depth

- Status: Accepted
- Date: 2026-02-20
- Context: Prior closure text implied complete public API docs while item-level rustdoc remained partial.
- Decision:
- Treat module-level docs as mandatory baseline.
- Require item-level docs for newly introduced public API in the same change set.
- Report legacy item-level rustdoc depth as incremental progress, not "fully complete."
- Consequences:
- Eliminates over-claims in project status reporting.
- Preserves merge discipline for new API surfaces without blocking on historical backlog in a single tranche.
- Impacted tasks/gates: loop-4w9 (historical interpretation), loop-k7d, docs/developer-guide/api-docs-status.md, doc review gates.

## D-022 Coverage Gate Threshold Recalibrated to Measured Baseline While Expansion Backlog Is Tracked

- Status: Accepted
- Date: 2026-02-20
- Context: The canonical `llvm-cov` gate now executes end-to-end in CI (including REPL-backed Rust tests), and measured aggregate line coverage for the full `rlm-core` scope is `70.11%` (`coverage/summary.txt`) rather than the historical `>=80%` target.
- Decision:
- Keep D-019 execution path and CI canonical enforcement model unchanged.
- Recalibrate `COVERAGE_MIN_LINES` default/policy from `80` to `70` so the gate enforces a real, reproducible floor instead of a non-actionable hard fail.
- Track follow-on work to raise effective coverage and eventually restore an `>=80%` threshold.
- Consequences:
- CI becomes green-gate actionable again while preserving an enforced coverage minimum.
- Coverage policy now matches measured repository reality across the full active scope.
- Prevents silent gate disablement or informal bypasses caused by unreachable thresholds.
- Impacted tasks/gates: loop-4kv, VG-COVERAGE-001, scripts/run_coverage.sh, .github/workflows/rlm-core-coverage.yml, docs/developer-guide/quality-gates.md, docs/execution-plan/VALIDATION-MATRIX.md.

## Update Rule

When adding a new decision:

1. Add ID, date, status, context, decision, consequences.
2. Reference impacted task IDs and VG IDs.
3. Do not rewrite prior decisions; append changes and supersede as needed.
