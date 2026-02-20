#!/usr/bin/env python3
"""Compare baseline/candidate perf runs and emit VG-PERF artifacts."""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


def parse_run_files(value: str) -> list[Path]:
    files = [Path(part.strip()) for part in value.split(",") if part.strip()]
    if not files:
        raise argparse.ArgumentTypeError("expected at least one JSON file")
    return files


def load_runs(paths: list[Path]) -> list[dict[str, Any]]:
    runs: list[dict[str, Any]] = []
    for path in paths:
        if not path.exists():
            raise FileNotFoundError(f"missing perf run file: {path}")
        runs.append(load_json(path))
    return runs


def nested_value(payload: dict[str, Any], *path: str) -> float:
    current: Any = payload
    for segment in path:
        current = current[segment]
    return float(current)


def aggregate_median(runs: list[dict[str, Any]], *path: str) -> float:
    values = [nested_value(run, *path) for run in runs]
    return statistics.median(values)


def commits_for(runs: list[dict[str, Any]]) -> list[str]:
    commits = {
        str(run.get("environment", {}).get("loop_commit", "unknown"))
        for run in runs
    }
    return sorted(commits)


def pct_change(baseline: float, candidate: float) -> float:
    if baseline == 0:
        return 0.0
    return ((candidate - baseline) / baseline) * 100.0


def latency_regression(baseline: float, candidate: float) -> float:
    # Positive means candidate is slower (worse).
    return pct_change(baseline, candidate)


def throughput_regression(baseline: float, candidate: float) -> float:
    # Positive means candidate throughput dropped (worse).
    if baseline == 0:
        return 0.0
    return ((baseline - candidate) / baseline) * 100.0


def latency_check(
    baseline: float,
    candidate: float,
    budget_pct: float,
    min_abs_latency_ms: float,
) -> dict[str, Any]:
    regression_pct = latency_regression(baseline, candidate)
    abs_delta_ms = candidate - baseline
    metric_pass = regression_pct <= budget_pct or abs_delta_ms <= min_abs_latency_ms
    return {
        "baseline_ms": baseline,
        "candidate_ms": candidate,
        "regression_pct": regression_pct,
        "abs_delta_ms": abs_delta_ms,
        "pass": metric_pass,
    }


def throughput_check(
    baseline: float,
    candidate: float,
    budget_pct: float,
    min_abs_drop_ops: float,
) -> dict[str, Any]:
    regression_pct = throughput_regression(baseline, candidate)
    abs_drop_ops_per_sec = baseline - candidate
    metric_pass = regression_pct <= budget_pct or abs_drop_ops_per_sec <= min_abs_drop_ops
    return {
        "baseline_ops_per_sec": baseline,
        "candidate_ops_per_sec": candidate,
        "regression_pct": regression_pct,
        "abs_drop_ops_per_sec": abs_drop_ops_per_sec,
        "pass": metric_pass,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--baseline",
        required=True,
        type=parse_run_files,
        help="Comma-separated baseline run JSON file(s)",
    )
    parser.add_argument(
        "--candidate",
        required=True,
        type=parse_run_files,
        help="Comma-separated candidate run JSON file(s)",
    )
    parser.add_argument("--vg-perf-001-out", required=True)
    parser.add_argument("--vg-perf-002-out", required=True)
    parser.add_argument("--summary-out", required=True)
    parser.add_argument("--budget-pct", type=float, default=10.0)
    parser.add_argument("--min-abs-latency-ms", type=float, default=2.0)
    parser.add_argument("--min-abs-throughput-drop-ops", type=float, default=150.0)
    parser.add_argument("--max-error-rate-delta", type=float, default=0.01)
    parser.add_argument(
        "--allow-same-commit",
        action="store_true",
        help="Allow baseline/candidate commit overlap (noise-calibration mode only)",
    )
    args = parser.parse_args()

    baseline_files: list[Path] = args.baseline
    candidate_files: list[Path] = args.candidate
    baseline_runs = load_runs(baseline_files)
    candidate_runs = load_runs(candidate_files)

    baseline_commits = commits_for(baseline_runs)
    candidate_commits = commits_for(candidate_runs)
    overlap = sorted(set(baseline_commits) & set(candidate_commits))
    if overlap and not args.allow_same_commit:
        print(
            "refusing same-commit perf comparison; provide a distinct baseline tuple "
            "or pass --allow-same-commit for calibration runs",
            file=sys.stderr,
        )
        print(f"overlapping commits: {', '.join(overlap)}", file=sys.stderr)
        raise SystemExit(2)

    budget = args.budget_pct

    perf001_checks = {
        "startup_p50": latency_check(
            aggregate_median(baseline_runs, "metrics", "startup_latency_ms", "p50_ms"),
            aggregate_median(candidate_runs, "metrics", "startup_latency_ms", "p50_ms"),
            budget,
            args.min_abs_latency_ms,
        ),
        "startup_p95": latency_check(
            aggregate_median(baseline_runs, "metrics", "startup_latency_ms", "p95_ms"),
            aggregate_median(candidate_runs, "metrics", "startup_latency_ms", "p95_ms"),
            budget,
            args.min_abs_latency_ms,
        ),
        "execute_no_submit_p50": latency_check(
            aggregate_median(
                baseline_runs, "metrics", "execute_latency_no_submit_ms", "p50_ms"
            ),
            aggregate_median(
                candidate_runs, "metrics", "execute_latency_no_submit_ms", "p50_ms"
            ),
            budget,
            args.min_abs_latency_ms,
        ),
        "execute_no_submit_p95": latency_check(
            aggregate_median(
                baseline_runs, "metrics", "execute_latency_no_submit_ms", "p95_ms"
            ),
            aggregate_median(
                candidate_runs, "metrics", "execute_latency_no_submit_ms", "p95_ms"
            ),
            budget,
            args.min_abs_latency_ms,
        ),
        "execute_with_submit_p50": latency_check(
            aggregate_median(
                baseline_runs, "metrics", "execute_latency_with_submit_ms", "p50_ms"
            ),
            aggregate_median(
                candidate_runs, "metrics", "execute_latency_with_submit_ms", "p50_ms"
            ),
            budget,
            args.min_abs_latency_ms,
        ),
        "execute_with_submit_p95": latency_check(
            aggregate_median(
                baseline_runs, "metrics", "execute_latency_with_submit_ms", "p95_ms"
            ),
            aggregate_median(
                candidate_runs, "metrics", "execute_latency_with_submit_ms", "p95_ms"
            ),
            budget,
            args.min_abs_latency_ms,
        ),
    }
    perf001_pass = all(check["pass"] for check in perf001_checks.values())

    baseline_throughput = aggregate_median(
        baseline_runs, "metrics", "batch", "throughput_ops_per_sec"
    )
    candidate_throughput = aggregate_median(
        candidate_runs, "metrics", "batch", "throughput_ops_per_sec"
    )
    baseline_error_rate = aggregate_median(baseline_runs, "metrics", "batch", "error_rate")
    candidate_error_rate = aggregate_median(
        candidate_runs, "metrics", "batch", "error_rate"
    )
    error_rate_delta = candidate_error_rate - baseline_error_rate
    perf002_checks = {
        "throughput": throughput_check(
            baseline_throughput,
            candidate_throughput,
            budget,
            args.min_abs_throughput_drop_ops,
        ),
        "error_rate_delta": {
            "baseline_error_rate": baseline_error_rate,
            "candidate_error_rate": candidate_error_rate,
            "delta": error_rate_delta,
            "max_allowed_delta": args.max_error_rate_delta,
            "pass": error_rate_delta <= args.max_error_rate_delta,
        },
    }
    perf002_pass = (
        perf002_checks["throughput"]["pass"] and perf002_checks["error_rate_delta"]["pass"]
    )

    vg_perf_001 = {
        "gate": "VG-PERF-001",
        "budget_pct": budget,
        "min_abs_latency_ms": args.min_abs_latency_ms,
        "aggregation": "median_across_runs",
        "baseline_files": [str(path) for path in baseline_files],
        "candidate_files": [str(path) for path in candidate_files],
        "baseline_commits": baseline_commits,
        "candidate_commits": candidate_commits,
        "allow_same_commit": args.allow_same_commit,
        "checks": perf001_checks,
        "pass": perf001_pass,
    }
    vg_perf_002 = {
        "gate": "VG-PERF-002",
        "budget_pct": budget,
        "min_abs_throughput_drop_ops": args.min_abs_throughput_drop_ops,
        "max_error_rate_delta": args.max_error_rate_delta,
        "aggregation": "median_across_runs",
        "baseline_files": [str(path) for path in baseline_files],
        "candidate_files": [str(path) for path in candidate_files],
        "baseline_commits": baseline_commits,
        "candidate_commits": candidate_commits,
        "allow_same_commit": args.allow_same_commit,
        "checks": perf002_checks,
        "baseline_throughput_ops_per_sec": baseline_throughput,
        "candidate_throughput_ops_per_sec": candidate_throughput,
        "baseline_error_rate": baseline_error_rate,
        "candidate_error_rate": candidate_error_rate,
        "pass": perf002_pass,
    }

    vg001_out = Path(args.vg_perf_001_out)
    vg002_out = Path(args.vg_perf_002_out)
    summary_out = Path(args.summary_out)
    vg001_out.parent.mkdir(parents=True, exist_ok=True)

    vg001_out.write_text(json.dumps(vg_perf_001, indent=2))
    vg002_out.write_text(json.dumps(vg_perf_002, indent=2))

    summary_lines = [
        "# M5-T01 Perf Comparison Summary",
        f"Budget: {budget:.1f}%",
        f"Latency floor: {args.min_abs_latency_ms:.2f} ms",
        f"Throughput floor: {args.min_abs_throughput_drop_ops:.2f} ops/sec",
        f"Max error-rate delta: {args.max_error_rate_delta:.4f}",
        f"VG-PERF-001: {'pass' if perf001_pass else 'fail'}",
        f"VG-PERF-002: {'pass' if perf002_pass else 'fail'}",
        f"Baseline run count: {len(baseline_files)}",
        f"Candidate run count: {len(candidate_files)}",
        f"Baseline commits: {', '.join(baseline_commits)}",
        f"Candidate commits: {', '.join(candidate_commits)}",
        f"Same-commit compare allowed: {args.allow_same_commit}",
        "",
        f"- baseline files: `{', '.join(str(path) for path in baseline_files)}`",
        f"- candidate files: `{', '.join(str(path) for path in candidate_files)}`",
        f"- vg-perf-001: `{args.vg_perf_001_out}`",
        f"- vg-perf-002: `{args.vg_perf_002_out}`",
    ]
    summary_out.write_text("\n".join(summary_lines) + "\n")

    print(f"Wrote {vg001_out}")
    print(f"Wrote {vg002_out}")
    print(f"Wrote {summary_out}")


if __name__ == "__main__":
    main()
