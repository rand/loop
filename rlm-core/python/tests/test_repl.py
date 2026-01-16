"""Tests for the RLM REPL."""

import pytest

from rlm_repl.deferred import (
    DeferredOperation,
    DeferredRegistry,
    OperationState,
    OperationType,
    PendingOperationError,
)
from rlm_repl.helpers import peek, search, count_tokens, truncate, extract_code_blocks
from rlm_repl.protocol import ExecuteRequest, ExecuteResponse, JsonRpcRequest, JsonRpcError
from rlm_repl.sandbox import Sandbox, SandboxError, CompilationError


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

    def test_accessing_pending_raises(self):
        sandbox = Sandbox()
        sandbox.execute("result = llm('test')")
        with pytest.raises(PendingOperationError):
            sandbox.execute("x = result.get()")
