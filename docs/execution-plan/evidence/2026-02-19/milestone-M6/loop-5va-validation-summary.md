# loop-5va Validation Summary (Canonical Reconciliation)

Date: 2026-02-19 (UTC)
Task: `loop-5va`
Mode: committed clean-clone tuple validation (`D-017`)

## Canonical State Observed

- Canonical repo: `/Users/rand/src/loop-agent`
- Canonical branch: `dp/loop-agent`
- Canonical committed SHA: `30c1fa786d79e0984cf464ffb8e67cc7a1bfcaeb`
- Canonical working tree: dirty (non-claim artifacts present), so claim runs remained clean-clone only.

## Validation Commands

1. Targeted candidate-related check:
   - `PYTHONPATH=/tmp/loop-agent-landing-check/src /Users/rand/src/loop-agent/.venv/bin/pytest -q tests/test_coverage_gaps.py::TestOptimizeMiprov2ImportError::test_miprov2_raises_import_error`
2. Seam gate snapshot (`VG-LA-001` equivalent subset):
   - `PYTHONPATH=/tmp/loop-agent-landing-check/src /Users/rand/src/loop-agent/.venv/bin/pytest -q tests/test_router.py tests/test_trajectory.py tests/test_set_backend_propagation.py tests/test_sensitivity_wiring.py`
3. Advisory full-suite snapshot (`VG-LA-002`):
   - `PYTHONPATH=/tmp/loop-agent-landing-check/src /Users/rand/src/loop-agent/.venv/bin/pytest -q`

All heavy runs were wrapped with `/Users/rand/src/loop/scripts/safe_run.sh` and `LOOP_MIN_AVAILABLE_MIB=4096`.

## Results

- Candidate-related targeted check: `1 passed`
- `VG-LA-001` subset: `30 passed`
- `VG-LA-002` advisory full suite: `936 passed`

Artifacts:

- `loop-5va-VG-LA-001-canonical30c1fa.txt`
- `loop-5va-VG-LA-002-canonical30c1fa.txt`

## Conclusion

- The original landing objective for candidate commit `f2aeb18` is superseded by newer canonical committed history.
- Canonical committed tuple `30c1fa` is now validated green in clean-clone mode for both seam and advisory full-suite snapshots.
- D-017 remains active to prevent claim drift from canonical dirty working-tree state.
