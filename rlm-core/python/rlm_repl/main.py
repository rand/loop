"""Main entry point for the RLM REPL subprocess.

This module implements a JSON-RPC server over stdin/stdout that provides
sandboxed Python execution with RLM helper functions.
"""

from __future__ import annotations

import json
import signal
import sys
import time
import traceback
from typing import Any

from rlm_repl.deferred import PendingOperationError, get_registry, reset_registry
from rlm_repl.protocol import (
    ErrorCode,
    ExecuteRequest,
    ExecuteResponse,
    GetVariableRequest,
    JsonRpcError,
    JsonRpcRequest,
    JsonRpcResponse,
    ResolveOperationRequest,
    SetVariableRequest,
    StatusResponse,
    VariablesResponse,
)
from rlm_repl.sandbox import CompilationError, Sandbox, SandboxError


class ReplServer:
    """JSON-RPC server for the REPL."""

    def __init__(self):
        self.sandbox = Sandbox()
        self.running = True

    def handle_request(self, request: JsonRpcRequest) -> JsonRpcResponse | None:
        """Handle a JSON-RPC request and return a response."""
        method = request.method
        params = request.params or {}

        try:
            if method == "execute":
                result = self._execute(params)
            elif method == "get_variable":
                result = self._get_variable(params)
            elif method == "set_variable":
                result = self._set_variable(params)
            elif method == "resolve_operation":
                result = self._resolve_operation(params)
            elif method == "list_variables":
                result = self._list_variables()
            elif method == "status":
                result = self._status()
            elif method == "reset":
                result = self._reset()
            elif method == "shutdown":
                self.running = False
                return JsonRpcResponse.success({"shutdown": True}, request.id)
            else:
                return JsonRpcResponse.failure(
                    JsonRpcError.method_not_found(method), request.id
                )

            return JsonRpcResponse.success(result, request.id)

        except Exception as e:
            error = JsonRpcError.execution_error(
                str(e),
                data={"type": type(e).__name__, "traceback": traceback.format_exc()},
            )
            return JsonRpcResponse.failure(error, request.id)

    def _execute(self, params: dict[str, Any]) -> dict[str, Any]:
        """Execute code in the sandbox."""
        req = ExecuteRequest(**params)

        start_time = time.perf_counter()
        pending_ops: list[str] = []

        try:
            result, stdout, stderr = self.sandbox.execute(req.code, req.capture_output)
            success = True
            error_msg = None
            error_type = None

        except PendingOperationError as e:
            # Code tried to access a pending deferred operation
            pending_ops.append(e.operation_id)
            result = None
            stdout = ""
            stderr = ""
            success = False
            error_msg = f"Pending operation: {e.operation_id}"
            error_type = "PendingOperationError"

        except CompilationError as e:
            result = None
            stdout = ""
            stderr = ""
            success = False
            error_msg = str(e)
            error_type = "CompilationError"

        except SandboxError as e:
            result = None
            stdout = ""
            stderr = ""
            success = False
            error_msg = str(e)
            error_type = "SandboxError"

        except Exception as e:
            result = None
            stdout = ""
            stderr = traceback.format_exc()
            success = False
            error_msg = str(e)
            error_type = type(e).__name__

        # Get all pending operations (not just the one that caused an error)
        all_pending = get_registry().pending_ids()

        execution_time_ms = (time.perf_counter() - start_time) * 1000

        return ExecuteResponse(
            success=success,
            result=_serialize_result(result),
            stdout=stdout,
            stderr=stderr,
            error=error_msg,
            error_type=error_type,
            execution_time_ms=execution_time_ms,
            pending_operations=all_pending,
        ).model_dump()

    def _get_variable(self, params: dict[str, Any]) -> Any:
        """Get a variable value."""
        req = GetVariableRequest(**params)
        value = self.sandbox.get_variable(req.name)
        return _serialize_result(value)

    def _set_variable(self, params: dict[str, Any]) -> dict[str, bool]:
        """Set a variable value."""
        req = SetVariableRequest(**params)
        self.sandbox.set_variable(req.name, req.value)
        return {"success": True}

    def _resolve_operation(self, params: dict[str, Any]) -> dict[str, bool]:
        """Resolve a deferred operation."""
        req = ResolveOperationRequest(**params)
        get_registry().resolve(req.operation_id, req.result)
        return {"success": True}

    def _list_variables(self) -> dict[str, Any]:
        """List all variables."""
        return VariablesResponse(variables=self.sandbox.list_variables()).model_dump()

    def _status(self) -> dict[str, Any]:
        """Get REPL status."""
        return StatusResponse(
            ready=True,
            pending_operations=len(get_registry().pending_ids()),
            variables_count=len(self.sandbox.list_variables()),
        ).model_dump()

    def _reset(self) -> dict[str, bool]:
        """Reset the REPL state."""
        self.sandbox = Sandbox()
        reset_registry()
        return {"success": True}

    def run(self) -> None:
        """Run the JSON-RPC server loop."""
        # Set up signal handlers
        signal.signal(signal.SIGTERM, lambda *_: setattr(self, "running", False))

        # Write ready message
        ready_msg = {"jsonrpc": "2.0", "method": "ready", "params": {"version": "0.1.0"}}
        sys.stdout.write(json.dumps(ready_msg) + "\n")
        sys.stdout.flush()

        while self.running:
            try:
                line = sys.stdin.readline()
                if not line:
                    # EOF - stdin closed
                    break

                line = line.strip()
                if not line:
                    continue

                # Parse request
                try:
                    data = json.loads(line)
                    request = JsonRpcRequest(**data)
                except json.JSONDecodeError as e:
                    error_response = JsonRpcResponse.failure(
                        JsonRpcError.parse_error(str(e))
                    )
                    sys.stdout.write(error_response.model_dump_json() + "\n")
                    sys.stdout.flush()
                    continue
                except Exception as e:
                    error_response = JsonRpcResponse.failure(
                        JsonRpcError.invalid_request(str(e))
                    )
                    sys.stdout.write(error_response.model_dump_json() + "\n")
                    sys.stdout.flush()
                    continue

                # Handle request
                response = self.handle_request(request)

                # Send response (unless it's a notification)
                if response is not None and not request.is_notification():
                    sys.stdout.write(response.model_dump_json() + "\n")
                    sys.stdout.flush()

            except KeyboardInterrupt:
                break
            except Exception as e:
                # Unexpected error - try to send error response
                try:
                    error_response = JsonRpcResponse.failure(
                        JsonRpcError(
                            code=ErrorCode.INTERNAL_ERROR,
                            message=f"Internal error: {e}",
                            data={"traceback": traceback.format_exc()},
                        )
                    )
                    sys.stdout.write(error_response.model_dump_json() + "\n")
                    sys.stdout.flush()
                except Exception:
                    pass


def _serialize_result(value: Any) -> Any:
    """Serialize a result value for JSON transmission."""
    if value is None:
        return None
    if isinstance(value, (str, int, float, bool)):
        return value
    if isinstance(value, (list, tuple)):
        return [_serialize_result(v) for v in value]
    if isinstance(value, dict):
        return {str(k): _serialize_result(v) for k, v in value.items()}
    if hasattr(value, "model_dump"):
        return value.model_dump()
    if hasattr(value, "__dict__"):
        return {k: _serialize_result(v) for k, v in value.__dict__.items() if not k.startswith("_")}
    return str(value)


def main() -> None:
    """Main entry point."""
    server = ReplServer()
    server.run()


if __name__ == "__main__":
    main()
