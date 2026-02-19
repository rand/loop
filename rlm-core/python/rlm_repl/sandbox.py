"""Sandboxed Python execution using RestrictedPython.

The sandbox provides a safe execution environment that:
- Blocks dangerous builtins (eval, exec, open, __import__)
- Restricts attribute access to safe patterns
- Limits available modules
- Enforces resource constraints via the host process
"""

from __future__ import annotations

import sys
from io import StringIO
from typing import Any

from RestrictedPython import compile_restricted, safe_builtins
from RestrictedPython.Eval import default_guarded_getiter
from RestrictedPython.Guards import (
    full_write_guard,
    guarded_iter_unpack_sequence,
    safer_getattr,
)

from rlm_repl import helpers
from rlm_repl.deferred import (
    DeferredOperation,
    DeferredRegistry,
    PendingOperationError,
    get_registry,
)


class SandboxError(Exception):
    """Error raised when sandbox detects a violation."""

    pass


class CompilationError(Exception):
    """Error raised when code fails to compile in restricted mode."""

    pass


class SubmitSignal(BaseException):
    """Internal control-flow signal to terminate execution after SUBMIT()."""


def _serialize_submit_value(value: Any) -> Any:
    """Serialize values to JSON-compatible structures for submit payloads."""
    if value is None or isinstance(value, (str, int, float, bool)):
        return value
    if isinstance(value, (list, tuple)):
        return [_serialize_submit_value(v) for v in value]
    if isinstance(value, dict):
        return {str(k): _serialize_submit_value(v) for k, v in value.items()}
    if hasattr(value, "model_dump"):
        return value.model_dump()
    if hasattr(value, "__dict__"):
        return {
            k: _serialize_submit_value(v)
            for k, v in value.__dict__.items()
            if not k.startswith("_")
        }
    return str(value)


def _preview_value(value: Any, limit: int = 100) -> str:
    """Create a bounded preview string for validation errors."""
    text = repr(value)
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."


def _type_name(value: Any) -> str:
    """Get the normalized type name used in validation errors."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    if isinstance(value, dict):
        return "object"
    return type(value).__name__


# Safe subset of builtins
SAFE_BUILTINS = {
    **safe_builtins,
    # Allow these additional builtins
    "dict": dict,
    "list": list,
    "set": set,
    "frozenset": frozenset,
    "tuple": tuple,
    "str": str,
    "int": int,
    "float": float,
    "bool": bool,
    "bytes": bytes,
    "type": type,
    "len": len,
    "range": range,
    "enumerate": enumerate,
    "zip": zip,
    "map": map,
    "filter": filter,
    "sorted": sorted,
    "reversed": reversed,
    "min": min,
    "max": max,
    "sum": sum,
    "any": any,
    "all": all,
    "abs": abs,
    "round": round,
    "pow": pow,
    "divmod": divmod,
    "isinstance": isinstance,
    "issubclass": issubclass,
    "hasattr": hasattr,
    "getattr": getattr,
    "repr": repr,
    "format": format,
    "chr": chr,
    "ord": ord,
    "hex": hex,
    "bin": bin,
    "oct": oct,
    "id": id,
    "hash": hash,
    "callable": callable,
    "iter": iter,
    "next": next,
    "slice": slice,
    "Exception": Exception,
    "ValueError": ValueError,
    "TypeError": TypeError,
    "KeyError": KeyError,
    "IndexError": IndexError,
    "AttributeError": AttributeError,
    "RuntimeError": RuntimeError,
    "StopIteration": StopIteration,
    # Explicitly blocked (override safe_builtins if needed)
    "eval": None,
    "exec": None,
    "compile": None,
    "open": None,
    "__import__": None,
    "input": None,
    "breakpoint": None,
}


# Remove None values (blocked builtins)
SAFE_BUILTINS = {k: v for k, v in SAFE_BUILTINS.items() if v is not None}


def _guarded_getattr(obj: Any, name: str) -> Any:
    """Custom getattr that blocks access to dangerous attributes."""
    # Block dunder access except for allowed ones
    if name.startswith("_"):
        allowed_dunders = {
            "__len__",
            "__iter__",
            "__next__",
            "__getitem__",
            "__contains__",
            "__str__",
            "__repr__",
            "__bool__",
            "__eq__",
            "__ne__",
            "__lt__",
            "__le__",
            "__gt__",
            "__ge__",
            "__hash__",
            "__add__",
            "__sub__",
            "__mul__",
            "__truediv__",
            "__floordiv__",
            "__mod__",
            "__pow__",
            "__neg__",
            "__pos__",
            "__abs__",
            "__class__",
            "__name__",
            "__doc__",
        }
        if name not in allowed_dunders:
            raise SandboxError(f"Access to '{name}' is not allowed")

    return safer_getattr(obj, name)


def _guarded_getitem(obj: Any, key: Any) -> Any:
    """Custom getitem that works with common types."""
    if isinstance(obj, (dict, list, tuple, str, bytes)):
        return obj[key]
    if hasattr(obj, "__getitem__"):
        return obj[key]
    raise TypeError(f"'{type(obj).__name__}' object is not subscriptable")


def _guarded_write(obj: Any) -> Any:
    """Guard for write operations."""
    return full_write_guard(obj)


def _guarded_import(
    name: str,
    globalz: dict | None = None,
    localz: dict | None = None,
    fromlist: tuple = (),
    level: int = 0,
) -> Any:
    """Restricted import that only allows safe modules."""
    allowed_modules = {
        "math",
        "re",
        "json",
        "collections",
        "itertools",
        "functools",
        "operator",
        "string",
        "textwrap",
        "datetime",
        "decimal",
        "fractions",
        "statistics",
        "random",  # Note: not cryptographically secure
        "copy",
        "pprint",
        "dataclasses",
        "typing",
        "enum",
        "abc",
    }

    if name not in allowed_modules:
        raise SandboxError(f"Import of '{name}' is not allowed")

    return __builtins__["__import__"](name, globalz, localz, fromlist, level)


class _PrintCollector:
    """Collector for RestrictedPython print() calls.

    RestrictedPython transforms print() calls to _print_() and expects
    a class that can collect the output.
    """

    def __init__(self, _getattr_=None):
        self._output: list[str] = []

    def _call_print(self, *args, **kwargs):
        """Handle a print() call."""
        sep = kwargs.get("sep", " ")
        end = kwargs.get("end", "\n")
        output = sep.join(str(arg) for arg in args) + end
        self._output.append(output)
        # Also write to actual stdout so capture works
        import sys
        sys.stdout.write(output)

    def __call__(self, *args, **kwargs):
        self._call_print(*args, **kwargs)
        return self

    @property
    def printed(self):
        return "".join(self._output)


class Sandbox:
    """Sandboxed Python execution environment."""

    def __init__(self, registry: DeferredRegistry | None = None):
        """Initialize the sandbox.

        Args:
            registry: DeferredRegistry for tracking async operations.
                     Uses global registry if not provided.
        """
        self.registry = registry or get_registry()
        self.signature_registration: dict[str, Any] | None = None
        self._submit_result: dict[str, Any] | None = None
        self._submit_count = 0
        self.globals: dict[str, Any] = {}
        self.locals: dict[str, Any] = {}
        self._setup_environment()

    def _setup_environment(self) -> None:
        """Set up the restricted execution environment."""
        # Restricted builtins
        self.globals["__builtins__"] = SAFE_BUILTINS.copy()

        # RestrictedPython guards
        self.globals["_getattr_"] = _guarded_getattr
        self.globals["_getitem_"] = _guarded_getitem
        self.globals["_getiter_"] = default_guarded_getiter
        self.globals["_iter_unpack_sequence_"] = guarded_iter_unpack_sequence
        self.globals["_write_"] = _guarded_write
        self.globals["__builtins__"]["__import__"] = _guarded_import

        # Print handler for RestrictedPython
        self.globals["_print_"] = _PrintCollector
        self.globals["_getattr_"] = _guarded_getattr

        # RLM helper functions
        self.globals["peek"] = helpers.peek
        self.globals["search"] = helpers.search
        self.globals["find_relevant"] = helpers.find_relevant
        self.globals["summarize"] = helpers.summarize
        self.globals["llm"] = helpers.llm
        self.globals["llm_batch"] = helpers.llm_batch
        self.globals["llm_query_batched"] = helpers.llm_query_batched
        self.globals["map_reduce"] = helpers.map_reduce
        self.globals["verify_claim"] = helpers.verify_claim
        self.globals["audit_reasoning"] = helpers.audit_reasoning
        self.globals["count_tokens"] = helpers.count_tokens
        self.globals["truncate"] = helpers.truncate
        self.globals["extract_code_blocks"] = helpers.extract_code_blocks
        self.globals["SUBMIT"] = self._submit

        # Expose DeferredOperation for type checking
        self.globals["DeferredOperation"] = DeferredOperation

    def compile(self, code: str, filename: str = "<repl>") -> Any:
        """Compile code in restricted mode.

        Args:
            code: Python source code
            filename: Filename for error messages

        Returns:
            Compiled code object

        Raises:
            CompilationError: If code fails to compile
        """
        try:
            # compile_restricted returns a code object directly in newer versions
            result = compile_restricted(code, filename, "exec")
            # Handle both old API (returns CompileResult) and new API (returns code)
            if hasattr(result, "errors") and result.errors:
                raise CompilationError("\n".join(result.errors))
            if hasattr(result, "code"):
                return result.code
            return result
        except SyntaxError as e:
            raise CompilationError(f"Syntax error: {e}")

    def set_signature_registration(self, registration: dict[str, Any] | None) -> None:
        """Set signature metadata used for SUBMIT validation."""
        self.signature_registration = registration

    def clear_signature_registration(self) -> None:
        """Clear signature metadata used for SUBMIT validation."""
        self.signature_registration = None

    def consume_submit_result(self) -> dict[str, Any] | None:
        """Return and clear the latest submit result for current execution."""
        result = self._submit_result
        self._submit_result = None
        return result

    def _reset_submit_state(self) -> None:
        self._submit_result = None
        self._submit_count = 0

    def _submit(self, outputs: Any) -> None:
        """SUBMIT callable exposed to sandboxed code."""
        serialized_outputs = _serialize_submit_value(outputs)
        self._submit_count += 1

        if self._submit_count > 1:
            self._submit_result = {
                "status": "validation_error",
                "errors": [
                    {
                        "error_type": "multiple_submits",
                        "count": self._submit_count,
                    }
                ],
                "original_outputs": serialized_outputs,
            }
            raise SubmitSignal()

        if self.signature_registration is None:
            self._submit_result = {
                "status": "validation_error",
                "errors": [{"error_type": "no_signature_registered"}],
                "original_outputs": serialized_outputs,
            }
            raise SubmitSignal()

        errors = self._validate_submit_outputs(serialized_outputs)
        if errors:
            self._submit_result = {
                "status": "validation_error",
                "errors": errors,
                "original_outputs": serialized_outputs,
            }
        else:
            self._submit_result = {
                "status": "success",
                "outputs": serialized_outputs,
            }
        raise SubmitSignal()

    def _validate_submit_outputs(self, outputs: Any) -> list[dict[str, Any]]:
        errors: list[dict[str, Any]] = []

        if not isinstance(outputs, dict):
            return [
                {
                    "error_type": "validation_failed",
                    "field": "",
                    "reason": "SUBMIT outputs must be an object",
                }
            ]

        output_fields = self.signature_registration.get("output_fields", [])
        for field_spec in output_fields:
            field_name = field_spec.get("name", "")
            field_type = field_spec.get("field_type", {"type": "custom", "value": "unknown"})
            required = field_spec.get("required", True)

            if required and field_name not in outputs:
                errors.append(
                    {
                        "error_type": "missing_field",
                        "field": field_name,
                        "expected_type": field_type,
                    }
                )
                continue

            if field_name in outputs:
                self._validate_field_value(field_name, field_type, outputs[field_name], errors)

        return errors

    def _validate_field_value(
        self,
        field_name: str,
        field_type: dict[str, Any],
        value: Any,
        errors: list[dict[str, Any]],
    ) -> None:
        type_tag = field_type.get("type")

        if type_tag == "string":
            if not isinstance(value, str):
                self._append_type_mismatch(errors, field_name, field_type, value)
            return

        if type_tag == "integer":
            if not isinstance(value, int) or isinstance(value, bool):
                self._append_type_mismatch(errors, field_name, field_type, value)
            return

        if type_tag == "float":
            if (not isinstance(value, (int, float))) or isinstance(value, bool):
                self._append_type_mismatch(errors, field_name, field_type, value)
            return

        if type_tag == "boolean":
            if not isinstance(value, bool):
                self._append_type_mismatch(errors, field_name, field_type, value)
            return

        if type_tag == "enum":
            allowed = field_type.get("value", [])
            if not isinstance(value, str):
                self._append_type_mismatch(errors, field_name, field_type, value)
            elif value not in allowed:
                errors.append(
                    {
                        "error_type": "enum_invalid",
                        "field": field_name,
                        "value": value,
                        "allowed": allowed,
                    }
                )
            return

        if type_tag == "list":
            inner_type = field_type.get("value", {"type": "custom", "value": "unknown"})
            if not isinstance(value, list):
                self._append_type_mismatch(errors, field_name, field_type, value)
                return
            for index, item in enumerate(value):
                self._validate_field_value(
                    f"{field_name}[{index}]",
                    inner_type,
                    item,
                    errors,
                )
            return

        if type_tag == "object":
            nested_fields = field_type.get("value", [])
            if not isinstance(value, dict):
                self._append_type_mismatch(errors, field_name, field_type, value)
                return
            for nested in nested_fields:
                nested_name = nested.get("name", "")
                nested_type = nested.get("field_type", {"type": "custom", "value": "unknown"})
                nested_required = nested.get("required", True)
                nested_path = f"{field_name}.{nested_name}" if nested_name else field_name

                if nested_required and nested_name not in value:
                    errors.append(
                        {
                            "error_type": "missing_field",
                            "field": nested_path,
                            "expected_type": nested_type,
                        }
                    )
                    continue

                if nested_name in value:
                    self._validate_field_value(
                        nested_path,
                        nested_type,
                        value[nested_name],
                        errors,
                    )
            return

        if type_tag == "custom":
            return

        errors.append(
            {
                "error_type": "validation_failed",
                "field": field_name,
                "reason": f"Unknown field type: {type_tag}",
            }
        )

    @staticmethod
    def _append_type_mismatch(
        errors: list[dict[str, Any]],
        field_name: str,
        expected_type: dict[str, Any],
        value: Any,
    ) -> None:
        errors.append(
            {
                "error_type": "type_mismatch",
                "field": field_name,
                "expected": expected_type,
                "got": _type_name(value),
                "value_preview": _preview_value(value),
            }
        )

    def execute(
        self,
        code: str,
        capture_output: bool = True,
    ) -> tuple[Any, str, str]:
        """Execute code in the sandbox.

        Args:
            code: Python source code to execute
            capture_output: Whether to capture stdout/stderr

        Returns:
            Tuple of (result, stdout, stderr)

        Raises:
            CompilationError: If code fails to compile
            SandboxError: If code violates sandbox restrictions
            PendingOperationError: If code accesses a pending deferred operation
        """
        self._reset_submit_state()
        compiled = self.compile(code)

        stdout_capture = StringIO() if capture_output else None
        stderr_capture = StringIO() if capture_output else None

        old_stdout = sys.stdout
        old_stderr = sys.stderr

        try:
            if capture_output:
                sys.stdout = stdout_capture  # type: ignore
                sys.stderr = stderr_capture  # type: ignore

            # Execute the code.
            # SUBMIT() raises SubmitSignal to terminate execution intentionally.
            try:
                exec(compiled, self.globals, self.locals)
            except SubmitSignal:
                pass

            # Get the result (last expression value, if any)
            result = self.locals.get("_", None)

            stdout = stdout_capture.getvalue() if stdout_capture else ""
            stderr = stderr_capture.getvalue() if stderr_capture else ""

            return result, stdout, stderr

        finally:
            sys.stdout = old_stdout
            sys.stderr = old_stderr

    def set_variable(self, name: str, value: Any) -> None:
        """Set a variable in the sandbox namespace."""
        if name.startswith("_") and name != "_":
            raise SandboxError(f"Cannot set variable with name '{name}'")
        self.locals[name] = value
        self.globals[name] = value

    def get_variable(self, name: str) -> Any:
        """Get a variable from the sandbox namespace."""
        if name in self.locals:
            return self.locals[name]
        if name in self.globals:
            return self.globals[name]
        raise KeyError(f"Variable '{name}' not found")

    def has_variable(self, name: str) -> bool:
        """Check if a variable exists."""
        return name in self.locals or name in self.globals

    def list_variables(self) -> dict[str, str]:
        """List all user variables with their types."""
        skip = {
            "__builtins__",
            "_getattr_",
            "_getitem_",
            "_getiter_",
            "_iter_unpack_sequence_",
            "_write_",
            "__import__",
            "DeferredOperation",
        }
        # Also skip helper functions
        skip.update(
            {
                "peek",
                "search",
                "find_relevant",
                "summarize",
                "llm",
                "llm_batch",
                "llm_query_batched",
                "map_reduce",
                "verify_claim",
                "audit_reasoning",
                "count_tokens",
                "truncate",
                "extract_code_blocks",
                "SUBMIT",
            }
        )

        variables = {}
        for name, value in {**self.globals, **self.locals}.items():
            if name not in skip and not name.startswith("_"):
                variables[name] = type(value).__name__
        return variables

    def clear(self) -> None:
        """Clear all user variables."""
        self.locals.clear()
        self._reset_submit_state()
        self._setup_environment()
