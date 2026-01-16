"""Deferred operations for async LLM calls.

The REPL runs synchronously, but LLM calls are inherently async. The DeferredOperation
pattern allows code to request an LLM call and receive a placeholder that will be
resolved by the host process.

Example:
    # In REPL code
    result = llm("Summarize this text", context=my_text)
    # result is a DeferredOperation, not the actual response

    # Later, after host resolves the operation:
    actual_result = result.get()  # Returns the resolved value
"""

from __future__ import annotations

import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable


class OperationType(str, Enum):
    """Types of deferred operations."""

    LLM_CALL = "llm_call"
    LLM_BATCH = "llm_batch"
    SUMMARIZE = "summarize"
    EMBED = "embed"
    MAP_REDUCE = "map_reduce"


class OperationState(str, Enum):
    """State of a deferred operation."""

    PENDING = "pending"
    RESOLVED = "resolved"
    FAILED = "failed"


@dataclass
class DeferredOperation:
    """A placeholder for an async operation result.

    DeferredOperations are returned by helper functions that need to make
    async calls (like LLM requests). The REPL execution pauses when code
    tries to access the result of a pending operation.
    """

    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    operation_type: OperationType = OperationType.LLM_CALL
    params: dict[str, Any] = field(default_factory=dict)
    state: OperationState = OperationState.PENDING
    result: Any = None
    error: str | None = None

    def is_pending(self) -> bool:
        """Check if the operation is still pending."""
        return self.state == OperationState.PENDING

    def is_resolved(self) -> bool:
        """Check if the operation has been resolved."""
        return self.state == OperationState.RESOLVED

    def is_failed(self) -> bool:
        """Check if the operation failed."""
        return self.state == OperationState.FAILED

    def resolve(self, result: Any) -> None:
        """Resolve the operation with a result."""
        if self.state != OperationState.PENDING:
            raise RuntimeError(f"Cannot resolve operation in state {self.state}")
        self.result = result
        self.state = OperationState.RESOLVED

    def fail(self, error: str) -> None:
        """Mark the operation as failed."""
        if self.state != OperationState.PENDING:
            raise RuntimeError(f"Cannot fail operation in state {self.state}")
        self.error = error
        self.state = OperationState.FAILED

    def get(self) -> Any:
        """Get the result, raising if pending or failed.

        This is the primary way code accesses deferred results. If the operation
        is still pending, it raises PendingOperationError which signals to the
        REPL runner that execution should pause.
        """
        if self.state == OperationState.PENDING:
            raise PendingOperationError(self.id)
        if self.state == OperationState.FAILED:
            raise DeferredOperationError(self.error or "Operation failed")
        return self.result

    def __repr__(self) -> str:
        return f"DeferredOperation({self.id[:8]}..., {self.operation_type.value}, {self.state.value})"

    # Make DeferredOperation behave like its result when resolved
    def __str__(self) -> str:
        if self.is_resolved():
            return str(self.result)
        return f"<Deferred:{self.id[:8]}>"

    def __bool__(self) -> bool:
        if self.is_resolved():
            return bool(self.result)
        raise PendingOperationError(self.id)

    def __len__(self) -> int:
        if self.is_resolved():
            return len(self.result)
        raise PendingOperationError(self.id)

    def __iter__(self):
        if self.is_resolved():
            return iter(self.result)
        raise PendingOperationError(self.id)


class PendingOperationError(Exception):
    """Raised when code tries to access a pending deferred operation.

    This is a control flow mechanism - the REPL runner catches this to know
    which operations need to be resolved before execution can continue.
    """

    def __init__(self, operation_id: str):
        self.operation_id = operation_id
        super().__init__(f"Operation {operation_id} is still pending")


class DeferredOperationError(Exception):
    """Raised when a deferred operation failed."""

    pass


class DeferredRegistry:
    """Registry for tracking deferred operations.

    The registry maintains all pending operations and provides methods for
    creating new operations and resolving existing ones.
    """

    def __init__(self):
        self._operations: dict[str, DeferredOperation] = {}
        self._on_operation_created: list[Callable[[DeferredOperation], None]] = []

    def create(
        self,
        operation_type: OperationType,
        params: dict[str, Any] | None = None,
    ) -> DeferredOperation:
        """Create a new deferred operation."""
        op = DeferredOperation(
            operation_type=operation_type,
            params=params or {},
        )
        self._operations[op.id] = op
        for callback in self._on_operation_created:
            callback(op)
        return op

    def get(self, operation_id: str) -> DeferredOperation | None:
        """Get an operation by ID."""
        return self._operations.get(operation_id)

    def resolve(self, operation_id: str, result: Any) -> None:
        """Resolve an operation with a result."""
        op = self._operations.get(operation_id)
        if op is None:
            raise KeyError(f"Unknown operation: {operation_id}")
        op.resolve(result)

    def fail(self, operation_id: str, error: str) -> None:
        """Mark an operation as failed."""
        op = self._operations.get(operation_id)
        if op is None:
            raise KeyError(f"Unknown operation: {operation_id}")
        op.fail(error)

    def pending_ids(self) -> list[str]:
        """Get IDs of all pending operations."""
        return [op.id for op in self._operations.values() if op.is_pending()]

    def pending_operations(self) -> list[DeferredOperation]:
        """Get all pending operations."""
        return [op for op in self._operations.values() if op.is_pending()]

    def clear_resolved(self) -> int:
        """Remove resolved and failed operations, return count removed."""
        to_remove = [
            op_id
            for op_id, op in self._operations.items()
            if not op.is_pending()
        ]
        for op_id in to_remove:
            del self._operations[op_id]
        return len(to_remove)

    def on_created(self, callback: Callable[[DeferredOperation], None]) -> None:
        """Register a callback for when operations are created."""
        self._on_operation_created.append(callback)

    def __len__(self) -> int:
        return len(self._operations)


# Global registry instance used by helper functions
_registry = DeferredRegistry()


def get_registry() -> DeferredRegistry:
    """Get the global deferred operation registry."""
    return _registry


def reset_registry() -> None:
    """Reset the global registry (for testing)."""
    global _registry
    _registry = DeferredRegistry()
