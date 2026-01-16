"""RLM REPL - Sandboxed Python execution for rlm-core.

This package provides a JSON-RPC based Python REPL that runs as a subprocess,
offering safe code execution with RLM-specific helper functions.
"""

from rlm_repl.deferred import DeferredOperation, DeferredRegistry
from rlm_repl.protocol import (
    ExecuteRequest,
    ExecuteResponse,
    JsonRpcError,
    JsonRpcRequest,
    JsonRpcResponse,
)

__version__ = "0.1.0"
__all__ = [
    "DeferredOperation",
    "DeferredRegistry",
    "ExecuteRequest",
    "ExecuteResponse",
    "JsonRpcError",
    "JsonRpcRequest",
    "JsonRpcResponse",
]
