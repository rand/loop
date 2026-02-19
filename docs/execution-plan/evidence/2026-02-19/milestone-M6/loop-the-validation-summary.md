# loop-the Validation Summary
Date: 2026-02-19
Task IDs: loop-the
VG IDs: VG-LA-001, VG-LA-002, VG-CONTRACT-001
Command(s):
- `safe_run.sh` + `uv run pytest -q tests/test_router.py tests/test_trajectory.py tests/test_set_backend_propagation.py tests/test_sensitivity_wiring.py`
- `safe_run.sh` + `uv run pytest -q` (loop-agent full-suite snapshot)
Result: partial-pass (seam-critical pass, advisory full-suite non-green)
Notes: Full-suite health improved substantially from prior baseline; remaining failures triaged and policy updated.

## Artifacts

- `loop-the-VG-LA-001.txt`
- `loop-the-VG-LA-002.txt`
- `loop-the-VG-LA-002-failure-summary.md`
- `loop-the-VG-CONTRACT-001.md`

## Outcomes

- Confirmed seam-critical compatibility remains stable.
- Reduced advisory full-suite failure envelope to two known tests.
- Added promotion criteria decision for future elevation of full-suite gate criticality.
