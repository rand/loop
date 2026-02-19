# Safe Start Checklist

Use this checklist at the start of every execution session.

## 1. Confirm Mode

- Confirm safe mode is active in `STATUS.md` and `WORKBOARD.md`.
- Confirm only one worker is authorized for heavy execution.

## 2. Run Memory Preflight

- `vm_stat | head -n 25`
- `memory_pressure | head -n 40`

## 3. Set Conservative Threshold

Use:

- `export LOOP_MIN_AVAILABLE_MIB=3072`

Lower only with explicit rationale in evidence.

## 4. Use Wrapper for Heavy Commands

Pattern:

- `/Users/rand/src/loop/scripts/safe_run.sh <heavy command>`

Examples:

- `/Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo check'`
- `/Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini signature::'`

## 5. Capture Safety Evidence

In each evidence artifact, include:

- threshold value
- wrapper admission/abort output
- mitigation actions (if any)

