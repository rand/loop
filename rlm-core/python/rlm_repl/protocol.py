"""JSON-RPC protocol types for REPL communication.

The REPL uses JSON-RPC 2.0 over stdin/stdout for communication with the
Rust host process. This module defines the message types.
"""

from __future__ import annotations

from enum import IntEnum
from typing import Any

from pydantic import BaseModel, Field


class ErrorCode(IntEnum):
    """JSON-RPC standard error codes."""

    PARSE_ERROR = -32700
    INVALID_REQUEST = -32600
    METHOD_NOT_FOUND = -32601
    INVALID_PARAMS = -32602
    INTERNAL_ERROR = -32603

    # Custom error codes (-32000 to -32099)
    EXECUTION_ERROR = -32000
    TIMEOUT_ERROR = -32001
    SANDBOX_VIOLATION = -32002
    RESOURCE_LIMIT = -32003


class JsonRpcError(BaseModel):
    """JSON-RPC error object."""

    code: int
    message: str
    data: Any | None = None

    @classmethod
    def parse_error(cls, message: str = "Parse error") -> JsonRpcError:
        return cls(code=ErrorCode.PARSE_ERROR, message=message)

    @classmethod
    def invalid_request(cls, message: str = "Invalid request") -> JsonRpcError:
        return cls(code=ErrorCode.INVALID_REQUEST, message=message)

    @classmethod
    def method_not_found(cls, method: str) -> JsonRpcError:
        return cls(code=ErrorCode.METHOD_NOT_FOUND, message=f"Method not found: {method}")

    @classmethod
    def execution_error(cls, message: str, data: Any | None = None) -> JsonRpcError:
        return cls(code=ErrorCode.EXECUTION_ERROR, message=message, data=data)

    @classmethod
    def timeout_error(cls, timeout_ms: int) -> JsonRpcError:
        return cls(
            code=ErrorCode.TIMEOUT_ERROR,
            message=f"Execution timed out after {timeout_ms}ms",
        )

    @classmethod
    def sandbox_violation(cls, message: str) -> JsonRpcError:
        return cls(code=ErrorCode.SANDBOX_VIOLATION, message=message)

    @classmethod
    def resource_limit(cls, resource: str, limit: str) -> JsonRpcError:
        return cls(
            code=ErrorCode.RESOURCE_LIMIT,
            message=f"Resource limit exceeded: {resource} (limit: {limit})",
        )


class JsonRpcRequest(BaseModel):
    """JSON-RPC 2.0 request."""

    jsonrpc: str = "2.0"
    method: str
    params: dict[str, Any] | list[Any] | None = None
    id: int | str | None = None

    def is_notification(self) -> bool:
        """Check if this is a notification (no id)."""
        return self.id is None


class JsonRpcResponse(BaseModel):
    """JSON-RPC 2.0 response."""

    jsonrpc: str = "2.0"
    result: Any | None = None
    error: JsonRpcError | None = None
    id: int | str | None = None

    @classmethod
    def success(cls, result: Any, request_id: int | str | None = None) -> JsonRpcResponse:
        return cls(result=result, id=request_id)

    @classmethod
    def failure(cls, error: JsonRpcError, request_id: int | str | None = None) -> JsonRpcResponse:
        return cls(error=error, id=request_id)


# REPL-specific request/response types


class ExecuteRequest(BaseModel):
    """Request to execute Python code in the REPL."""

    code: str = Field(..., description="Python code to execute")
    timeout_ms: int = Field(default=30000, description="Execution timeout in milliseconds")
    capture_output: bool = Field(default=True, description="Whether to capture stdout/stderr")


class ExecuteResponse(BaseModel):
    """Response from code execution."""

    success: bool
    result: Any | None = Field(default=None, description="Return value if any")
    stdout: str = Field(default="", description="Captured stdout")
    stderr: str = Field(default="", description="Captured stderr")
    error: str | None = Field(default=None, description="Error message if failed")
    error_type: str | None = Field(default=None, description="Exception type if failed")
    execution_time_ms: float = Field(default=0, description="Execution time in milliseconds")
    pending_operations: list[str] = Field(
        default_factory=list, description="IDs of pending deferred operations"
    )
    submit_result: dict[str, Any] | None = Field(
        default=None,
        description=(
            "Result of SUBMIT call if execution used typed-signature submission"
        ),
    )


class GetVariableRequest(BaseModel):
    """Request to get a variable value."""

    name: str


class SetVariableRequest(BaseModel):
    """Request to set a variable value."""

    name: str
    value: Any


class ResolveOperationRequest(BaseModel):
    """Request to resolve a deferred operation."""

    operation_id: str
    result: Any


class RegisterSignatureRequest(BaseModel):
    """Request to register output signature metadata for SUBMIT validation."""

    output_fields: list[dict[str, Any]] = Field(
        ..., description="Output field specifications"
    )
    signature_name: str | None = Field(
        default=None, description="Optional signature label for diagnostics"
    )


class VariablesResponse(BaseModel):
    """Response listing available variables."""

    variables: dict[str, str] = Field(
        default_factory=dict, description="Variable names to type descriptions"
    )


class StatusResponse(BaseModel):
    """Response with REPL status."""

    ready: bool
    pending_operations: int
    variables_count: int
    signature_registered: bool = False
    memory_usage_bytes: int | None = None
