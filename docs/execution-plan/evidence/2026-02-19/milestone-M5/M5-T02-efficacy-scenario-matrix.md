# M5-T02 Efficacy Scenario Matrix
Date: 2026-02-19
Scope: Typed-signature and fallback behavior efficacy scenarios

## Scenario Classes

| Scenario Class | Coverage Test(s) | Notes |
|---|---|---|
| Valid submit | `tests/test_repl.py::TestReplServer::test_submit_with_registered_signature_success` | Confirms structured `submit_result` success payload |
| Validation failure: no signature | `tests/test_repl.py::TestReplServer::test_submit_without_signature_returns_validation_error` | Confirms `no_signature_registered` structured error |
| Validation failure: missing field | `tests/test_repl.py::TestReplServer::test_submit_missing_field_returns_structured_error` | Confirms required-field enforcement |
| Validation failure: type mismatch | `tests/test_repl.py::TestReplServer::test_submit_type_mismatch_returns_structured_error` | Confirms field type validation |
| Validation failure: multiple submits | `tests/test_repl.py::TestReplServer::test_multiple_submit_calls_return_structured_error` | Confirms single-submit contract |
| Fallback/non-submit behavior | `tests/test_repl.py::TestReplServer::test_execute_without_submit_when_signature_registered` | Confirms execution succeeds with `submit_result=None` when no SUBMIT is called |
| Batch behavior under mixed success/failure | `tests/test_repl.py::TestReplServer::test_llm_batch_mixed_success_failure_resolution` | Confirms mixed payload resolution path is deterministic and retrievable |

## Gate Command

`LOOP_MIN_AVAILABLE_MIB=4096 /Users/rand/src/loop/scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core/python && uv run pytest -q tests/test_repl.py'`
