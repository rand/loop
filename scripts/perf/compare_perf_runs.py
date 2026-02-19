#!/usr/bin/env python3
"""Compare baseline/candidate perf runs and emit VG-PERF artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text())


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


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--vg-perf-001-out", required=True)
    parser.add_argument("--vg-perf-002-out", required=True)
    parser.add_argument("--summary-out", required=True)
    parser.add_argument("--budget-pct", type=float, default=10.0)
    args = parser.parse_args()

    baseline = load_json(Path(args.baseline))
    candidate = load_json(Path(args.candidate))
    budget = args.budget_pct

    b_metrics = baseline["metrics"]
    c_metrics = candidate["metrics"]

    perf001_checks = {
        "startup_p50": latency_regression(
            b_metrics["startup_latency_ms"]["p50_ms"],
            c_metrics["startup_latency_ms"]["p50_ms"],
        ),
        "startup_p95": latency_regression(
            b_metrics["startup_latency_ms"]["p95_ms"],
            c_metrics["startup_latency_ms"]["p95_ms"],
        ),
        "execute_no_submit_p50": latency_regression(
            b_metrics["execute_latency_no_submit_ms"]["p50_ms"],
            c_metrics["execute_latency_no_submit_ms"]["p50_ms"],
        ),
        "execute_no_submit_p95": latency_regression(
            b_metrics["execute_latency_no_submit_ms"]["p95_ms"],
            c_metrics["execute_latency_no_submit_ms"]["p95_ms"],
        ),
        "execute_with_submit_p50": latency_regression(
            b_metrics["execute_latency_with_submit_ms"]["p50_ms"],
            c_metrics["execute_latency_with_submit_ms"]["p50_ms"],
        ),
        "execute_with_submit_p95": latency_regression(
            b_metrics["execute_latency_with_submit_ms"]["p95_ms"],
            c_metrics["execute_latency_with_submit_ms"]["p95_ms"],
        ),
    }
    perf001_pass = all(v <= budget for v in perf001_checks.values())

    b_batch = b_metrics["batch"]
    c_batch = c_metrics["batch"]
    perf002_checks = {
        "throughput_regression_pct": throughput_regression(
            b_batch["throughput_ops_per_sec"],
            c_batch["throughput_ops_per_sec"],
        ),
        "error_rate_delta": c_batch["error_rate"] - b_batch["error_rate"],
    }
    perf002_pass = (
        perf002_checks["throughput_regression_pct"] <= budget
        and perf002_checks["error_rate_delta"] <= 0.01
    )

    vg_perf_001 = {
        "gate": "VG-PERF-001",
        "budget_pct": budget,
        "baseline_file": args.baseline,
        "candidate_file": args.candidate,
        "checks": perf001_checks,
        "pass": perf001_pass,
    }
    vg_perf_002 = {
        "gate": "VG-PERF-002",
        "budget_pct": budget,
        "baseline_file": args.baseline,
        "candidate_file": args.candidate,
        "checks": perf002_checks,
        "baseline_throughput_ops_per_sec": b_batch["throughput_ops_per_sec"],
        "candidate_throughput_ops_per_sec": c_batch["throughput_ops_per_sec"],
        "baseline_error_rate": b_batch["error_rate"],
        "candidate_error_rate": c_batch["error_rate"],
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
        f"VG-PERF-001: {'pass' if perf001_pass else 'fail'}",
        f"VG-PERF-002: {'pass' if perf002_pass else 'fail'}",
        "",
        f"- baseline: `{args.baseline}`",
        f"- candidate: `{args.candidate}`",
        f"- vg-perf-001: `{args.vg_perf_001_out}`",
        f"- vg-perf-002: `{args.vg_perf_002_out}`",
    ]
    summary_out.write_text("\n".join(summary_lines) + "\n")

    print(f"Wrote {vg001_out}")
    print(f"Wrote {vg002_out}")
    print(f"Wrote {summary_out}")


if __name__ == "__main__":
    main()
