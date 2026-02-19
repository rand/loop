#!/usr/bin/env python3
"""Measure REPL startup, execute latency, and synthetic batch throughput."""

from __future__ import annotations

import argparse
import json
import platform
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path("/Users/rand/src/loop")
PY_REPL_DIR = ROOT / "rlm-core" / "python"


def percentile(values: list[float], pct: float) -> float:
    if not values:
        return 0.0
    if len(values) == 1:
        return values[0]
    ordered = sorted(values)
    rank = (len(ordered) - 1) * pct
    lower = int(rank)
    upper = min(lower + 1, len(ordered) - 1)
    if lower == upper:
        return ordered[lower]
    weight = rank - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def metric_summary(values: list[float]) -> dict[str, Any]:
    if not values:
        return {
            "count": 0,
            "min_ms": 0.0,
            "max_ms": 0.0,
            "mean_ms": 0.0,
            "p50_ms": 0.0,
            "p95_ms": 0.0,
        }
    return {
        "count": len(values),
        "min_ms": min(values),
        "max_ms": max(values),
        "mean_ms": statistics.fmean(values),
        "p50_ms": percentile(values, 0.50),
        "p95_ms": percentile(values, 0.95),
    }


def git_rev(path: Path) -> str:
    try:
        return (
            subprocess.check_output(["git", "-C", str(path), "rev-parse", "HEAD"], text=True)
            .strip()
        )
    except Exception:
        return "unknown"


@dataclass
class ReplClient:
    proc: subprocess.Popen[str]
    next_id: int = 1

    @classmethod
    def start(cls) -> tuple["ReplClient", float]:
        start = time.perf_counter()
        proc = subprocess.Popen(
            [sys.executable, "-m", "rlm_repl"],
            cwd=PY_REPL_DIR,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        assert proc.stdout is not None
        line = proc.stdout.readline()
        elapsed_ms = (time.perf_counter() - start) * 1000.0
        if not line:
            raise RuntimeError("REPL exited before ready message")
        payload = json.loads(line)
        if payload.get("method") != "ready":
            raise RuntimeError(f"Unexpected first message: {payload}")
        return cls(proc=proc), elapsed_ms

    def request(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        req_id = self.next_id
        self.next_id += 1
        msg = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params or {},
        }
        assert self.proc.stdin is not None
        assert self.proc.stdout is not None
        self.proc.stdin.write(json.dumps(msg) + "\n")
        self.proc.stdin.flush()

        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError(f"No response for method={method}, id={req_id}")
            payload = json.loads(line)
            if payload.get("id") != req_id:
                continue
            if payload.get("error") is not None:
                raise RuntimeError(f"RPC error for {method}: {payload['error']}")
            return payload["result"]

    def stop(self) -> None:
        try:
            self.request("shutdown", {})
        except Exception:
            pass
        try:
            self.proc.wait(timeout=2.0)
        except Exception:
            self.proc.kill()
            self.proc.wait(timeout=2.0)


def benchmark_startup(iterations: int) -> list[float]:
    samples: list[float] = []
    for _ in range(iterations):
        client, startup_ms = ReplClient.start()
        samples.append(startup_ms)
        client.stop()
    return samples


def signature_params() -> dict[str, Any]:
    return {
        "output_fields": [
            {
                "name": "answer",
                "field_type": {"type": "string"},
                "description": "Final answer",
                "prefix": None,
                "required": True,
                "default": None,
            }
        ],
        "signature_name": "PerfSubmitSig",
    }


def benchmark_execute(iterations: int, with_submit: bool) -> list[float]:
    client, _ = ReplClient.start()
    try:
        if with_submit:
            client.request("register_signature", signature_params())

        samples: list[float] = []
        code = "SUBMIT({'answer': 'ok'})" if with_submit else "x = 1 + 1\nx"
        for _ in range(iterations):
            start = time.perf_counter()
            result = client.request("execute", {"code": code, "capture_output": True})
            elapsed_ms = (time.perf_counter() - start) * 1000.0
            if not result.get("success", False):
                raise RuntimeError(f"Execute failed: {result}")
            samples.append(elapsed_ms)
        return samples
    finally:
        client.stop()


def benchmark_batch(iterations: int, batch_size: int) -> dict[str, Any]:
    client, _ = ReplClient.start()
    try:
        samples: list[float] = []
        errors = 0
        total_cycles = 0
        prompts = [f"prompt-{i}" for i in range(batch_size)]
        contexts = [f"context-{i}" for i in range(batch_size)]
        code = f"op = llm_batch({prompts!r}, contexts={contexts!r}, max_parallel=5)"

        for idx in range(iterations):
            total_cycles += 1
            start = time.perf_counter()
            exec_result = client.request("execute", {"code": code, "capture_output": True})
            pending = exec_result.get("pending_operations", [])
            if not exec_result.get("success", False) or not pending:
                errors += 1
                continue

            op_id = pending[0]
            # Alternate plain and mixed payloads to emulate mixed batch outcomes.
            if idx % 2 == 0:
                resolved = [f"result-{j}" for j in range(batch_size)]
            else:
                resolved = [
                    ({"status": "ok", "value": f"result-{j}"} if j % 2 == 0 else {"status": "error", "value": "synthetic-failure"})
                    for j in range(batch_size)
                ]
            client.request("resolve_operation", {"operation_id": op_id, "result": resolved})
            elapsed_ms = (time.perf_counter() - start) * 1000.0
            samples.append(elapsed_ms)

        total_time_s = sum(samples) / 1000.0 if samples else 0.0
        throughput_ops_per_sec = (len(samples) / total_time_s) if total_time_s > 0 else 0.0
        error_rate = (errors / total_cycles) if total_cycles > 0 else 0.0

        return {
            "cycle_latency_ms": metric_summary(samples),
            "successful_cycles": len(samples),
            "total_cycles": total_cycles,
            "error_count": errors,
            "error_rate": error_rate,
            "batch_size": batch_size,
            "throughput_ops_per_sec": throughput_ops_per_sec,
        }
    finally:
        client.stop()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--label", required=True, help="Run label, e.g. baseline/candidate")
    parser.add_argument("--output", required=True, help="Output JSON path")
    parser.add_argument("--startup-iters", type=int, default=15)
    parser.add_argument("--exec-iters", type=int, default=80)
    parser.add_argument("--submit-iters", type=int, default=80)
    parser.add_argument("--batch-iters", type=int, default=80)
    parser.add_argument("--batch-size", type=int, default=8)
    args = parser.parse_args()

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    startup_samples = benchmark_startup(args.startup_iters)
    exec_samples = benchmark_execute(args.exec_iters, with_submit=False)
    submit_samples = benchmark_execute(args.submit_iters, with_submit=True)
    batch_metrics = benchmark_batch(args.batch_iters, args.batch_size)

    payload = {
        "label": args.label,
        "timestamp_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "environment": {
            "platform": platform.platform(),
            "python_version": platform.python_version(),
            "processor": platform.processor(),
            "loop_commit": git_rev(ROOT),
            "cwd": str(PY_REPL_DIR),
        },
        "config": {
            "startup_iters": args.startup_iters,
            "exec_iters": args.exec_iters,
            "submit_iters": args.submit_iters,
            "batch_iters": args.batch_iters,
            "batch_size": args.batch_size,
        },
        "metrics": {
            "startup_latency_ms": metric_summary(startup_samples),
            "execute_latency_no_submit_ms": metric_summary(exec_samples),
            "execute_latency_with_submit_ms": metric_summary(submit_samples),
            "batch": batch_metrics,
        },
    }

    output_path.write_text(json.dumps(payload, indent=2))
    print(f"Wrote {output_path}")


if __name__ == "__main__":
    main()
