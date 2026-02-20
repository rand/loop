"""Tests for the RLM REPL."""

import pytest

from rlm_repl.deferred import (
    DeferredOperation,
    DeferredRegistry,
    OperationState,
    OperationType,
    PendingOperationError,
)
from rlm_repl.helpers import (
    count_tokens,
    extract_code_blocks,
    llm_batch,
    llm_query_batched,
    peek,
    search,
    truncate,
)
from rlm_repl.main import ReplServer
from rlm_repl.protocol import (
    ErrorCode,
    ExecuteRequest,
    ExecuteResponse,
    JsonRpcError,
    JsonRpcRequest,
)
from rlm_repl.sandbox import CompilationError, Sandbox, SandboxError


class TestDeferredOperations:
    """Tests for the deferred operation system."""

    def test_create_operation(self):
        registry = DeferredRegistry()
        op = registry.create(OperationType.LLM_CALL, {"prompt": "test"})

        assert op.is_pending()
        assert not op.is_resolved()
        assert op.operation_type == OperationType.LLM_CALL
        assert op.params["prompt"] == "test"

    def test_resolve_operation(self):
        registry = DeferredRegistry()
        op = registry.create(OperationType.LLM_CALL)

        registry.resolve(op.id, "result value")

        assert op.is_resolved()
        assert op.get() == "result value"

    def test_fail_operation(self):
        registry = DeferredRegistry()
        op = registry.create(OperationType.LLM_CALL)

        registry.fail(op.id, "test error")

        assert op.is_failed()
        with pytest.raises(Exception, match="test error"):
            op.get()

    def test_pending_operation_raises(self):
        op = DeferredOperation()
        with pytest.raises(PendingOperationError):
            op.get()

    def test_pending_ids(self):
        registry = DeferredRegistry()
        op1 = registry.create(OperationType.LLM_CALL)
        op2 = registry.create(OperationType.SUMMARIZE)

        assert len(registry.pending_ids()) == 2

        registry.resolve(op1.id, "done")

        assert len(registry.pending_ids()) == 1
        assert op2.id in registry.pending_ids()


class TestHelpers:
    """Tests for helper functions."""

    def test_peek_string(self):
        text = "line1\nline2\nline3\nline4\nline5"
        result = peek(text, 1, 3)
        assert result == "line2\nline3"

    def test_peek_list(self):
        data = [1, 2, 3, 4, 5]
        result = peek(data, 0, 2)
        assert "[1, 2]" in result

    def test_search_string(self):
        text = "foo bar\nbaz foo\nqux"
        matches = search(text, "foo")
        assert len(matches) == 2
        assert matches[0]["index"] == 0
        assert matches[1]["index"] == 1

    def test_search_regex(self):
        text = "error: something\nwarning: other\nerror: again"
        matches = search(text, r"error:.*", regex=True)
        assert len(matches) == 2

    def test_search_case_insensitive(self):
        text = "Hello\nhello\nHELLO"
        matches = search(text, "hello", case_sensitive=False)
        assert len(matches) == 3

    def test_count_tokens(self):
        text = "This is a test string"
        tokens = count_tokens(text)
        assert tokens > 0
        assert tokens == len(text) // 4

    def test_truncate(self):
        text = "a" * 10000
        truncated = truncate(text, max_tokens=100)
        assert len(truncated) < len(text)
        assert truncated.endswith("...")

    def test_extract_code_blocks(self):
        text = """
Some text
```python
def foo():
    pass
```
More text
```javascript
console.log('hi')
```
"""
        blocks = extract_code_blocks(text)
        assert len(blocks) == 2
        assert blocks[0]["language"] == "python"
        assert "def foo" in blocks[0]["code"]
        assert blocks[1]["language"] == "javascript"

    def test_llm_batch_helper_params(self):
        op = llm_batch(
            prompts=["q1", "q2"],
            contexts=["c1", "c2"],
            max_parallel=7,
            model="test-model",
            max_tokens=321,
        )
        assert isinstance(op, DeferredOperation)
        assert op.operation_type == OperationType.LLM_BATCH
        assert op.params["prompts"] == ["q1", "q2"]
        assert op.params["contexts"] == ["c1", "c2"]
        assert op.params["max_parallel"] == 7
        assert op.params["model"] == "test-model"
        assert op.params["max_tokens"] == 321

    def test_llm_query_batched_alias_deprecated(self):
        with pytest.warns(DeprecationWarning):
            op = llm_query_batched(["q1"], max_parallel=3)
        assert isinstance(op, DeferredOperation)
        assert op.operation_type == OperationType.LLM_BATCH
        assert op.params["max_parallel"] == 3


class TestProtocol:
    """Tests for JSON-RPC protocol types."""

    def test_execute_request(self):
        req = ExecuteRequest(code="1 + 1")
        assert req.code == "1 + 1"
        assert req.timeout_ms == 30000
        assert req.capture_output is True

    def test_execute_response(self):
        resp = ExecuteResponse(
            success=True,
            result=42,
            stdout="",
            stderr="",
        )
        assert resp.success
        assert resp.result == 42

    def test_jsonrpc_request(self):
        req = JsonRpcRequest(method="execute", params={"code": "x"}, id=1)
        assert req.jsonrpc == "2.0"
        assert not req.is_notification()

    def test_jsonrpc_notification(self):
        req = JsonRpcRequest(method="notify", params={})
        assert req.is_notification()

    def test_error_codes(self):
        error = JsonRpcError.execution_error("test")
        assert error.code == -32000
        assert error.message == "test"


class TestReplServer:
    """Tests for JSON-RPC method handling in ReplServer."""

    @staticmethod
    def _signature_params():
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
            "signature_name": "AnswerSig",
        }

    def test_register_and_clear_signature(self):
        server = ReplServer()

        register_req = JsonRpcRequest(
            method="register_signature", params=self._signature_params(), id=1
        )
        register_resp = server.handle_request(register_req)

        assert register_resp is not None
        assert register_resp.error is None
        assert register_resp.result["success"] is True
        assert register_resp.result["signature_registered"] is True
        assert register_resp.result["replaced"] is False
        assert server.signature_registration is not None

        clear_req = JsonRpcRequest(method="clear_signature", params={}, id=2)
        clear_resp = server.handle_request(clear_req)

        assert clear_resp is not None
        assert clear_resp.error is None
        assert clear_resp.result["success"] is True
        assert clear_resp.result["cleared"] is True
        assert server.signature_registration is None

    def test_clear_signature_is_idempotent(self):
        server = ReplServer()

        clear_req = JsonRpcRequest(method="clear_signature", params={}, id=1)
        clear_resp = server.handle_request(clear_req)

        assert clear_resp is not None
        assert clear_resp.error is None
        assert clear_resp.result["success"] is True
        assert clear_resp.result["cleared"] is False

    def test_register_signature_invalid_params(self):
        server = ReplServer()

        invalid_req = JsonRpcRequest(
            method="register_signature", params={"signature_name": "MissingFields"}, id=1
        )
        resp = server.handle_request(invalid_req)

        assert resp is not None
        assert resp.error is not None
        assert resp.error.code == ErrorCode.INVALID_PARAMS

    def test_status_reports_signature_registration(self):
        server = ReplServer()

        status_req = JsonRpcRequest(method="status", params={}, id=1)
        status_resp = server.handle_request(status_req)
        assert status_resp is not None
        assert status_resp.error is None
        assert status_resp.result["signature_registered"] is False

        register_req = JsonRpcRequest(
            method="register_signature", params=self._signature_params(), id=2
        )
        server.handle_request(register_req)

        status_after_req = JsonRpcRequest(method="status", params={}, id=3)
        status_after_resp = server.handle_request(status_after_req)
        assert status_after_resp is not None
        assert status_after_resp.error is None
        assert status_after_resp.result["signature_registered"] is True

    def test_submit_without_signature_returns_validation_error(self):
        server = ReplServer()

        req = JsonRpcRequest(
            method="execute",
            params={"code": "SUBMIT({'answer': 'test'})"},
            id=1,
        )
        resp = server.handle_request(req)

        assert resp is not None
        assert resp.error is None
        assert resp.result["success"] is False
        assert resp.result["error_type"] == "SubmitValidationError"
        submit_result = resp.result["submit_result"]
        assert submit_result["status"] == "validation_error"
        assert submit_result["errors"][0]["error_type"] == "no_signature_registered"

    def test_submit_with_registered_signature_success(self):
        server = ReplServer()
        server.handle_request(
            JsonRpcRequest(method="register_signature", params=self._signature_params(), id=1)
        )

        req = JsonRpcRequest(
            method="execute",
            params={"code": "SUBMIT({'answer': 'test'})"},
            id=2,
        )
        resp = server.handle_request(req)

        assert resp is not None
        assert resp.error is None
        assert resp.result["success"] is True
        submit_result = resp.result["submit_result"]
        assert submit_result["status"] == "success"
        assert submit_result["outputs"]["answer"] == "test"

    def test_submit_missing_field_returns_structured_error(self):
        server = ReplServer()
        server.handle_request(
            JsonRpcRequest(method="register_signature", params=self._signature_params(), id=1)
        )

        req = JsonRpcRequest(
            method="execute",
            params={"code": "SUBMIT({})"},
            id=2,
        )
        resp = server.handle_request(req)

        assert resp is not None
        assert resp.error is None
        assert resp.result["success"] is False
        submit_result = resp.result["submit_result"]
        assert submit_result["status"] == "validation_error"
        assert submit_result["errors"][0]["error_type"] == "missing_field"
        assert submit_result["errors"][0]["field"] == "answer"

    def test_submit_type_mismatch_returns_structured_error(self):
        server = ReplServer()
        server.handle_request(
            JsonRpcRequest(method="register_signature", params=self._signature_params(), id=1)
        )

        req = JsonRpcRequest(
            method="execute",
            params={"code": "SUBMIT({'answer': 42})"},
            id=2,
        )
        resp = server.handle_request(req)

        assert resp is not None
        assert resp.error is None
        assert resp.result["success"] is False
        submit_result = resp.result["submit_result"]
        assert submit_result["status"] == "validation_error"
        assert submit_result["errors"][0]["error_type"] == "type_mismatch"
        assert submit_result["errors"][0]["field"] == "answer"

    def test_multiple_submit_calls_return_structured_error(self):
        server = ReplServer()
        server.handle_request(
            JsonRpcRequest(method="register_signature", params=self._signature_params(), id=1)
        )

        code = """
try:
    SUBMIT({'answer': 'first'})
except BaseException:
    pass
SUBMIT({'answer': 'second'})
"""
        req = JsonRpcRequest(
            method="execute",
            params={"code": code},
            id=2,
        )
        resp = server.handle_request(req)

        assert resp is not None
        assert resp.error is None
        assert resp.result["success"] is False
        submit_result = resp.result["submit_result"]
        assert submit_result["status"] == "validation_error"
        assert submit_result["errors"][0]["error_type"] == "multiple_submits"
        assert submit_result["errors"][0]["count"] == 2

    def test_execute_without_submit_when_signature_registered(self):
        server = ReplServer()
        server.handle_request(JsonRpcRequest(method="reset", params={}, id=0))
        server.handle_request(
            JsonRpcRequest(method="register_signature", params=self._signature_params(), id=1)
        )

        req = JsonRpcRequest(
            method="execute",
            params={"code": "value = 123\nvalue"},
            id=2,
        )
        resp = server.handle_request(req)

        assert resp is not None
        assert resp.error is None
        assert resp.result["success"] is True
        assert resp.result["submit_result"] is None
        value_resp = server.handle_request(
            JsonRpcRequest(method="get_variable", params={"name": "value"}, id=3)
        )
        assert value_resp is not None
        assert value_resp.error is None
        assert value_resp.result == 123

    def test_llm_batch_mixed_success_failure_resolution(self):
        server = ReplServer()
        server.handle_request(JsonRpcRequest(method="reset", params={}, id=0))

        create_req = JsonRpcRequest(
            method="execute",
            params={"code": "op = llm_batch(['q1', 'q2'])"},
            id=1,
        )
        create_resp = server.handle_request(create_req)

        assert create_resp is not None
        assert create_resp.error is None
        assert create_resp.result["success"] is True
        pending = create_resp.result["pending_operations"]
        assert len(pending) >= 1
        op_id = pending[-1]

        mixed_payload = [
            {"status": "success", "value": "answer-1"},
            {"status": "error", "value": "timeout"},
        ]
        resolve_req = JsonRpcRequest(
            method="resolve_operation",
            params={"operation_id": op_id, "result": mixed_payload},
            id=2,
        )
        resolve_resp = server.handle_request(resolve_req)
        assert resolve_resp is not None
        assert resolve_resp.error is None
        assert resolve_resp.result["success"] is True

        read_req = JsonRpcRequest(method="execute", params={"code": "resolved = op.get()"}, id=3)
        read_resp = server.handle_request(read_req)
        assert read_resp is not None
        assert read_resp.error is None
        assert read_resp.result["success"] is True
        resolved_resp = server.handle_request(
            JsonRpcRequest(method="get_variable", params={"name": "resolved"}, id=4)
        )
        assert resolved_resp is not None
        assert resolved_resp.error is None
        resolved = resolved_resp.result
        assert resolved[0]["status"] == "success"
        assert resolved[1]["status"] == "error"

    def test_pending_operations_exposes_operation_metadata(self):
        server = ReplServer()
        server.handle_request(JsonRpcRequest(method="reset", params={}, id=0))

        create_req = JsonRpcRequest(
            method="execute",
            params={"code": "op = llm_batch(['q1', 'q2'], max_parallel=3)"},
            id=1,
        )
        create_resp = server.handle_request(create_req)

        assert create_resp is not None
        assert create_resp.error is None
        assert create_resp.result["success"] is True
        assert len(create_resp.result["pending_operations"]) >= 1

        pending_req = JsonRpcRequest(method="pending_operations", params={}, id=2)
        pending_resp = server.handle_request(pending_req)

        assert pending_resp is not None
        assert pending_resp.error is None
        operations = pending_resp.result["operations"]
        assert len(operations) >= 1
        op = operations[-1]
        assert op["operation_type"] == OperationType.LLM_BATCH.value
        assert op["params"]["prompts"] == ["q1", "q2"]
        assert op["params"]["max_parallel"] == 3


class TestSandbox:
    """Tests for the sandbox execution environment."""

    def test_simple_execution(self):
        sandbox = Sandbox()
        result, stdout, stderr = sandbox.execute("x = 1 + 1")
        assert "x" in sandbox.list_variables()

    def test_stdout_capture(self):
        sandbox = Sandbox()
        _, stdout, _ = sandbox.execute("print('hello')")
        assert "hello" in stdout

    def test_variable_access(self):
        sandbox = Sandbox()
        sandbox.set_variable("test_var", 42)
        assert sandbox.get_variable("test_var") == 42

    def test_blocked_builtins(self):
        sandbox = Sandbox()
        with pytest.raises((CompilationError, SandboxError, NameError)):
            sandbox.execute("open('/etc/passwd')")

    def test_blocked_import(self):
        sandbox = Sandbox()
        with pytest.raises(SandboxError, match="not allowed"):
            sandbox.execute("import os")

    def test_allowed_import(self):
        sandbox = Sandbox()
        result, _, _ = sandbox.execute("import math; x = math.sqrt(4)")
        assert sandbox.get_variable("x") == 2.0

    def test_helper_functions_available(self):
        sandbox = Sandbox()
        result, _, _ = sandbox.execute("result = peek('line1\\nline2', 0, 1)")
        assert sandbox.get_variable("result") == "line1"

    def test_dunder_access_blocked(self):
        sandbox = Sandbox()
        with pytest.raises((SandboxError, CompilationError)):
            sandbox.execute("x = ().__class__.__bases__[0].__subclasses__()")

    def test_list_variables(self):
        sandbox = Sandbox()
        sandbox.execute("a = 1; b = 'hello'; c = [1, 2, 3]")
        variables = sandbox.list_variables()
        assert "a" in variables
        assert "b" in variables
        assert "c" in variables
        assert variables["a"] == "int"
        assert variables["b"] == "str"

    def test_clear(self):
        sandbox = Sandbox()
        sandbox.execute("x = 42")
        assert "x" in sandbox.list_variables()
        sandbox.clear()
        assert "x" not in sandbox.list_variables()


class TestDeferredInSandbox:
    """Tests for deferred operations within sandbox execution."""

    def test_llm_returns_deferred(self):
        sandbox = Sandbox()
        sandbox.execute("result = llm('test prompt')")
        result = sandbox.get_variable("result")
        assert isinstance(result, DeferredOperation)
        assert result.is_pending()

    def test_summarize_returns_deferred(self):
        sandbox = Sandbox()
        sandbox.execute("result = summarize('some text to summarize')")
        result = sandbox.get_variable("result")
        assert isinstance(result, DeferredOperation)
        assert result.operation_type == OperationType.SUMMARIZE

    def test_llm_query_batched_alias_available(self):
        sandbox = Sandbox()
        sandbox.execute("result = llm_query_batched(['q1', 'q2'])")
        result = sandbox.get_variable("result")
        assert isinstance(result, DeferredOperation)
        assert result.operation_type == OperationType.LLM_BATCH

    def test_accessing_pending_raises(self):
        sandbox = Sandbox()
        sandbox.execute("result = llm('test')")
        with pytest.raises(PendingOperationError):
            sandbox.execute("x = result.get()")
