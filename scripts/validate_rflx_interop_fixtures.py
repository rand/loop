#!/usr/bin/env python3
"""Validate io-rflx interop fixtures and confidence calibration policy."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

CONFIDENCE_ORDER = ["Speculative", "Plausible", "High", "Verified"]
MODEL_TIERS = ["Simple", "Standard", "Complex"]
OUTCOME_STATUSES = ["Verified", "Rejected", "Accepted", "Proposed", "Error"]
VERIFICATION_OUTCOMES = ["Verified", "Unproved", "Timeout", "Error"]


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def require_path(obj: dict[str, Any], keys: list[str], fixture: str) -> Any:
    current: Any = obj
    trail: list[str] = []
    for key in keys:
        trail.append(key)
        if not isinstance(current, dict) or key not in current:
            joined = ".".join(trail)
            raise ValueError(f"{fixture}: missing required field '{joined}'")
        current = current[key]
    return current


def require_schema_version(obj: dict[str, Any], fixture: str) -> None:
    value = require_path(obj, ["schema_version"], fixture)
    if value != "io_rflx_interop.v0":
        raise ValueError(
            f"{fixture}: schema_version must be 'io_rflx_interop.v0', got '{value}'"
        )


def confidence_bucket(score: float, thresholds: dict[str, float]) -> str:
    if score < thresholds["speculative"]:
        return "Speculative"
    if score < thresholds["plausible"]:
        return "Plausible"
    if score < thresholds["high"]:
        return "High"
    return "Verified"


def validate_thresholds(thresholds: dict[str, float]) -> None:
    keys = ["speculative", "plausible", "high"]
    for key in keys:
        if key not in thresholds:
            raise ValueError(f"calibration: missing threshold '{key}'")
        value = thresholds[key]
        if not isinstance(value, (int, float)):
            raise ValueError(f"calibration: threshold '{key}' must be numeric")
    if not (0.0 <= thresholds["speculative"] < thresholds["plausible"] < thresholds["high"] <= 1.0):
        raise ValueError("calibration: thresholds must satisfy 0 <= speculative < plausible < high <= 1")


def validate_provenance_fixture(data: dict[str, Any]) -> None:
    fixture = "provenance-envelope.json"
    require_schema_version(data, fixture)
    payload_type = require_path(data, ["payload_type"], fixture)
    if payload_type != "provenance_envelope":
        raise ValueError(f"{fixture}: payload_type must be 'provenance_envelope'")

    source = require_path(data, ["payload", "source"], fixture)
    if not isinstance(source, dict) or len(source) != 1:
        raise ValueError(f"{fixture}: payload.source must be a single-variant object")

    confidence = require_path(data, ["payload", "confidence"], fixture)
    if confidence not in CONFIDENCE_ORDER:
        raise ValueError(
            f"{fixture}: payload.confidence must be one of {CONFIDENCE_ORDER}, got '{confidence}'"
        )

    created_at = require_path(data, ["payload", "created_at"], fixture)
    if not isinstance(created_at, int) or created_at <= 0:
        raise ValueError(f"{fixture}: payload.created_at must be a positive integer")

    evidence = require_path(data, ["payload", "evidence"], fixture)
    if not isinstance(evidence, list):
        raise ValueError(f"{fixture}: payload.evidence must be a list")


def validate_trajectory_fixture(data: dict[str, Any]) -> None:
    fixture = "trajectory-envelope.json"
    require_schema_version(data, fixture)
    payload_type = require_path(data, ["payload_type"], fixture)
    if payload_type != "trajectory_envelope":
        raise ValueError(f"{fixture}: payload_type must be 'trajectory_envelope'")

    require_path(data, ["payload", "id"], fixture)
    require_path(data, ["payload", "session_id"], fixture)
    timestamp = require_path(data, ["payload", "timestamp"], fixture)
    if not isinstance(timestamp, int) or timestamp <= 0:
        raise ValueError(f"{fixture}: payload.timestamp must be a positive integer")

    hole_type = require_path(data, ["payload", "routing", "hole_type"], fixture)
    if hole_type not in ["Expression", "Statement", "Block", "Function", "Module"]:
        raise ValueError(f"{fixture}: unsupported routing.hole_type '{hole_type}'")

    chosen_tier = require_path(data, ["payload", "routing", "chosen_tier"], fixture)
    if chosen_tier not in MODEL_TIERS:
        raise ValueError(f"{fixture}: routing.chosen_tier must be one of {MODEL_TIERS}")

    for path in [
        ["payload", "routing", "hole_id"],
        ["payload", "routing", "constraint_count"],
        ["payload", "routing", "complexity_score"],
        ["payload", "generation", "provider"],
        ["payload", "generation", "model"],
        ["payload", "generation", "prompt_tokens"],
        ["payload", "generation", "completion_tokens"],
        ["payload", "generation", "latency_ms"],
        ["payload", "generation", "estimated_cost_usd"],
        ["payload", "outcome", "status"],
    ]:
        require_path(data, path, fixture)

    status = require_path(data, ["payload", "outcome", "status"], fixture)
    if status not in OUTCOME_STATUSES:
        raise ValueError(f"{fixture}: outcome.status must be one of {OUTCOME_STATUSES}")


def validate_calibration_cases(data: dict[str, Any]) -> dict[str, float]:
    fixture = "confidence-calibration-cases.json"
    require_schema_version(data, fixture)

    method = require_path(data, ["method"], fixture)
    if method != "confidence_bucket_v1":
        raise ValueError(f"{fixture}: method must be 'confidence_bucket_v1'")

    thresholds = require_path(data, ["thresholds"], fixture)
    if not isinstance(thresholds, dict):
        raise ValueError(f"{fixture}: thresholds must be an object")
    validate_thresholds(thresholds)

    cases = require_path(data, ["cases"], fixture)
    if not isinstance(cases, list) or not cases:
        raise ValueError(f"{fixture}: cases must be a non-empty list")

    for index, case in enumerate(cases):
        if not isinstance(case, dict):
            raise ValueError(f"{fixture}: case #{index} must be an object")

        for key in ["loop_status", "p1", "expected"]:
            if key not in case:
                raise ValueError(f"{fixture}: case #{index} missing '{key}'")

        p1 = case["p1"]
        if not isinstance(p1, (int, float)):
            raise ValueError(f"{fixture}: case #{index} p1 must be numeric")
        if not 0.0 <= float(p1) <= 1.0:
            raise ValueError(f"{fixture}: case #{index} p1 out of [0,1]")

        expected = case["expected"]
        if expected not in CONFIDENCE_ORDER:
            raise ValueError(f"{fixture}: case #{index} expected '{expected}' is invalid")

        actual = confidence_bucket(float(p1), thresholds)
        if actual != expected:
            raise ValueError(
                f"{fixture}: case #{index} expected '{expected}' but bucketed to '{actual}'"
            )

    return {k: float(v) for k, v in thresholds.items()}


def validate_verification_fixture(data: dict[str, Any], thresholds: dict[str, float]) -> None:
    fixture = "verification-envelope.json"
    require_schema_version(data, fixture)

    payload_type = require_path(data, ["payload_type"], fixture)
    if payload_type != "verification_envelope":
        raise ValueError(f"{fixture}: payload_type must be 'verification_envelope'")

    model_loaded = require_path(data, ["payload", "model_loaded"], fixture)
    if not isinstance(model_loaded, bool):
        raise ValueError(f"{fixture}: payload.model_loaded must be boolean")

    outcomes = require_path(data, ["payload", "outcomes"], fixture)
    if not isinstance(outcomes, list) or not outcomes:
        raise ValueError(f"{fixture}: payload.outcomes must be a non-empty list")

    for index, outcome in enumerate(outcomes):
        if not isinstance(outcome, dict):
            raise ValueError(f"{fixture}: outcome #{index} must be an object")
        for key in ["statement", "critical", "outcome"]:
            if key not in outcome:
                raise ValueError(f"{fixture}: outcome #{index} missing '{key}'")
        if outcome["outcome"] not in VERIFICATION_OUTCOMES:
            raise ValueError(
                f"{fixture}: outcome #{index} has invalid state '{outcome['outcome']}'"
            )

    confidence = require_path(data, ["payload", "confidence"], fixture)
    if not isinstance(confidence, (int, float)) or not 0.0 <= float(confidence) <= 1.0:
        raise ValueError(f"{fixture}: payload.confidence must be in [0,1]")

    elapsed = require_path(data, ["payload", "total_elapsed_ms"], fixture)
    if not isinstance(elapsed, int) or elapsed < 0:
        raise ValueError(f"{fixture}: payload.total_elapsed_ms must be a non-negative integer")

    method = require_path(data, ["calibration", "method"], fixture)
    if method != "confidence_bucket_v1":
        raise ValueError(f"{fixture}: calibration.method must be 'confidence_bucket_v1'")

    expected_bucket = require_path(data, ["calibration", "expected_bucket"], fixture)
    if expected_bucket not in CONFIDENCE_ORDER:
        raise ValueError(
            f"{fixture}: calibration.expected_bucket must be one of {CONFIDENCE_ORDER}"
        )

    max_distance = require_path(data, ["calibration", "max_bucket_distance"], fixture)
    if not isinstance(max_distance, int) or max_distance < 0:
        raise ValueError(f"{fixture}: calibration.max_bucket_distance must be a non-negative integer")

    actual_bucket = confidence_bucket(float(confidence), thresholds)
    expected_index = CONFIDENCE_ORDER.index(expected_bucket)
    actual_index = CONFIDENCE_ORDER.index(actual_bucket)
    if abs(actual_index - expected_index) > max_distance:
        raise ValueError(
            f"{fixture}: confidence bucket '{actual_bucket}' exceeds max distance {max_distance} from expected '{expected_bucket}'"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate io-rflx interop fixtures.")
    parser.add_argument(
        "--fixtures-dir",
        type=Path,
        default=Path(
            "docs/execution-plan/contracts/fixtures/io-rflx/io_rflx_interop.v0"
        ),
        help="Path to fixture directory",
    )
    args = parser.parse_args()

    fixtures_dir = args.fixtures_dir
    required_files = {
        "provenance": fixtures_dir / "provenance-envelope.json",
        "trajectory": fixtures_dir / "trajectory-envelope.json",
        "verification": fixtures_dir / "verification-envelope.json",
        "calibration": fixtures_dir / "confidence-calibration-cases.json",
    }

    for label, path in required_files.items():
        if not path.exists():
            raise FileNotFoundError(f"missing required fixture '{label}': {path}")

    provenance = load_json(required_files["provenance"])
    trajectory = load_json(required_files["trajectory"])
    verification = load_json(required_files["verification"])
    calibration = load_json(required_files["calibration"])

    thresholds = validate_calibration_cases(calibration)
    validate_provenance_fixture(provenance)
    validate_trajectory_fixture(trajectory)
    validate_verification_fixture(verification, thresholds)

    print("io-rflx interop fixtures: PASS")
    print(f"fixtures_dir={fixtures_dir}")
    print("schema_version=io_rflx_interop.v0")
    print("calibration_method=confidence_bucket_v1")
    print(f"calibration_cases={len(calibration['cases'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
