"""REPL helper functions for RLM operations.

These functions are available in the REPL sandbox and provide the core
RLM capabilities: peeking at data, searching, and making LLM calls.

All LLM-related functions return DeferredOperations that must be resolved
by the host process.
"""

from __future__ import annotations

import re
import warnings
from typing import Any, Sequence

from rlm_repl.deferred import DeferredOperation, OperationType, get_registry


def peek(data: Any, start: int = 0, end: int | None = None) -> str:
    """Peek at a slice of data.

    Works with strings, lists, and other sequences. Returns a string
    representation of the slice.

    Args:
        data: The data to peek at
        start: Start index (default 0)
        end: End index (default None = to end)

    Returns:
        String representation of the slice

    Examples:
        >>> peek(conversation, 0, 5)  # First 5 messages
        >>> peek(file_content, 100, 200)  # Lines 100-200
    """
    if isinstance(data, str):
        lines = data.splitlines()
        sliced = lines[start:end]
        return "\n".join(sliced)
    elif isinstance(data, (list, tuple)):
        sliced = data[start:end]
        if all(isinstance(item, str) for item in sliced):
            return "\n".join(sliced)
        return repr(sliced)
    elif hasattr(data, "__getitem__"):
        return repr(data[start:end])
    else:
        return repr(data)


def search(
    data: Any,
    pattern: str,
    regex: bool = False,
    case_sensitive: bool = True,
    context_lines: int = 0,
) -> list[dict[str, Any]]:
    """Search for a pattern in data.

    Returns matches with their locations. For strings, returns line numbers.
    For lists, returns indices.

    Args:
        data: The data to search in
        pattern: Pattern to search for
        regex: Whether pattern is a regex (default False)
        case_sensitive: Whether search is case-sensitive (default True)
        context_lines: Number of context lines around matches (default 0)

    Returns:
        List of match dictionaries with 'index', 'content', and optionally 'context'

    Examples:
        >>> search(file_content, "def ")  # Find all function definitions
        >>> search(logs, r"error.*timeout", regex=True, case_sensitive=False)
    """
    matches = []
    flags = 0 if case_sensitive else re.IGNORECASE

    if regex:
        compiled = re.compile(pattern, flags)
    else:
        escaped = re.escape(pattern)
        compiled = re.compile(escaped, flags)

    if isinstance(data, str):
        lines = data.splitlines()
        for i, line in enumerate(lines):
            if compiled.search(line):
                match_info: dict[str, Any] = {"index": i, "content": line}
                if context_lines > 0:
                    start = max(0, i - context_lines)
                    end = min(len(lines), i + context_lines + 1)
                    match_info["context"] = "\n".join(lines[start:end])
                matches.append(match_info)
    elif isinstance(data, (list, tuple)):
        for i, item in enumerate(data):
            item_str = str(item) if not isinstance(item, str) else item
            if compiled.search(item_str):
                matches.append({"index": i, "content": item})
    elif isinstance(data, dict):
        for key, value in data.items():
            key_str = str(key)
            value_str = str(value)
            if compiled.search(key_str) or compiled.search(value_str):
                matches.append({"key": key, "content": value})

    return matches


def find_relevant(
    data: Any,
    query: str,
    top_k: int = 5,
) -> DeferredOperation:
    """Find the most relevant chunks of data for a query.

    Uses semantic similarity to rank chunks. Returns a deferred operation
    that resolves to a list of relevant chunks.

    Args:
        data: The data to search in
        query: The query to find relevant content for
        top_k: Number of results to return (default 5)

    Returns:
        DeferredOperation that resolves to list of relevant chunks
    """
    # Prepare data as chunks
    if isinstance(data, str):
        chunks = _chunk_text(data)
    elif isinstance(data, list):
        chunks = [str(item) for item in data]
    else:
        chunks = [str(data)]

    return get_registry().create(
        OperationType.EMBED,
        params={
            "query": query,
            "chunks": chunks,
            "top_k": top_k,
        },
    )


def _chunk_text(text: str, chunk_size: int = 500, overlap: int = 50) -> list[str]:
    """Split text into overlapping chunks."""
    lines = text.splitlines()
    chunks = []
    current_chunk: list[str] = []
    current_size = 0

    for line in lines:
        line_size = len(line)
        if current_size + line_size > chunk_size and current_chunk:
            chunks.append("\n".join(current_chunk))
            # Keep overlap
            overlap_lines = []
            overlap_size = 0
            for prev_line in reversed(current_chunk):
                if overlap_size + len(prev_line) > overlap:
                    break
                overlap_lines.insert(0, prev_line)
                overlap_size += len(prev_line)
            current_chunk = overlap_lines
            current_size = overlap_size

        current_chunk.append(line)
        current_size += line_size

    if current_chunk:
        chunks.append("\n".join(current_chunk))

    return chunks


def summarize(
    data: Any,
    max_tokens: int = 500,
    focus: str | None = None,
) -> DeferredOperation:
    """Summarize data using an LLM.

    Args:
        data: The data to summarize
        max_tokens: Maximum tokens in the summary (default 500)
        focus: Optional focus area for the summary

    Returns:
        DeferredOperation that resolves to the summary string
    """
    content = str(data) if not isinstance(data, str) else data

    prompt = f"Summarize the following in at most {max_tokens} tokens"
    if focus:
        prompt += f", focusing on {focus}"
    prompt += f":\n\n{content}"

    return get_registry().create(
        OperationType.SUMMARIZE,
        params={
            "content": content,
            "max_tokens": max_tokens,
            "focus": focus,
            "prompt": prompt,
        },
    )


def llm(
    prompt: str,
    context: str | None = None,
    model: str | None = None,
    max_tokens: int = 1024,
    temperature: float = 0.0,
) -> DeferredOperation:
    """Make an LLM call.

    Args:
        prompt: The prompt to send to the LLM
        context: Optional additional context
        model: Optional model override (default uses routing)
        max_tokens: Maximum response tokens (default 1024)
        temperature: Sampling temperature (default 0.0)

    Returns:
        DeferredOperation that resolves to the LLM response string
    """
    return get_registry().create(
        OperationType.LLM_CALL,
        params={
            "prompt": prompt,
            "context": context,
            "model": model,
            "max_tokens": max_tokens,
            "temperature": temperature,
        },
    )


def llm_batch(
    prompts: Sequence[str],
    contexts: Sequence[str] | None = None,
    max_parallel: int = 5,
    model: str | None = None,
    max_tokens: int = 1024,
) -> DeferredOperation:
    """Make parallel LLM calls.

    Args:
        prompts: List of prompts
        contexts: Optional list of contexts (same length as prompts)
        max_parallel: Maximum concurrent queries (default 5)
        model: Optional model override
        max_tokens: Maximum response tokens per call

    Returns:
        DeferredOperation that resolves to list of response strings
    """
    if contexts is not None and len(contexts) != len(prompts):
        raise ValueError("contexts must have same length as prompts")

    return get_registry().create(
        OperationType.LLM_BATCH,
        params={
            "prompts": list(prompts),
            "contexts": list(contexts) if contexts else None,
            "max_parallel": max_parallel,
            "model": model,
            "max_tokens": max_tokens,
        },
    )


def llm_query_batched(
    prompts: Sequence[str],
    contexts: Sequence[str] | None = None,
    max_parallel: int = 5,
    model: str | None = None,
    max_tokens: int = 1024,
) -> DeferredOperation:
    """Compatibility alias for llm_batch().

    Deprecated:
        Use `llm_batch(...)` instead.
    """
    warnings.warn(
        "llm_query_batched() is deprecated; use llm_batch() instead.",
        DeprecationWarning,
        stacklevel=2,
    )
    return llm_batch(
        prompts=prompts,
        contexts=contexts,
        max_parallel=max_parallel,
        model=model,
        max_tokens=max_tokens,
    )


def map_reduce(
    data: Sequence[Any],
    map_prompt: str,
    reduce_prompt: str,
    chunk_size: int = 10,
) -> DeferredOperation:
    """Apply map-reduce pattern over data using LLM.

    First maps each chunk through the map_prompt, then reduces all
    results using the reduce_prompt.

    Args:
        data: Sequence of items to process
        map_prompt: Prompt template for mapping (use {item} placeholder)
        reduce_prompt: Prompt template for reducing (use {results} placeholder)
        chunk_size: Number of items per map call (default 10)

    Returns:
        DeferredOperation that resolves to the final reduced result
    """
    # Chunk the data
    items = list(data)
    chunks = [items[i : i + chunk_size] for i in range(0, len(items), chunk_size)]

    return get_registry().create(
        OperationType.MAP_REDUCE,
        params={
            "chunks": chunks,
            "map_prompt": map_prompt,
            "reduce_prompt": reduce_prompt,
        },
    )


# Verification helpers (Strawberry integration)


def verify_claim(
    claim: str,
    evidence: str,
    confidence: float = 0.95,
) -> DeferredOperation:
    """Verify a claim against evidence using Strawberry methodology.

    Args:
        claim: The claim to verify
        evidence: The evidence to check against
        confidence: Target confidence level (default 0.95)

    Returns:
        DeferredOperation that resolves to a verification result dict
        with p0, p1, required_bits, observed_bits, budget_gap, status
    """
    return get_registry().create(
        OperationType.LLM_CALL,
        params={
            "type": "verify_claim",
            "claim": claim,
            "evidence": evidence,
            "target_confidence": confidence,
        },
    )


def audit_reasoning(
    steps: Sequence[str],
    sources: Sequence[str] | None = None,
) -> DeferredOperation:
    """Audit a reasoning trace for hallucinations.

    Args:
        steps: List of reasoning steps
        sources: Optional list of source materials

    Returns:
        DeferredOperation that resolves to list of step verifications
    """
    return get_registry().create(
        OperationType.LLM_CALL,
        params={
            "type": "audit_reasoning",
            "steps": list(steps),
            "sources": list(sources) if sources else None,
        },
    )


# Utility helpers


def count_tokens(text: str) -> int:
    """Approximate token count for text.

    Uses rough heuristic: ~4 chars per token for English.
    """
    return len(text) // 4


def truncate(text: str, max_tokens: int = 1000) -> str:
    """Truncate text to approximately max_tokens."""
    max_chars = max_tokens * 4
    if len(text) <= max_chars:
        return text
    return text[: max_chars - 3] + "..."


def extract_code_blocks(text: str) -> list[dict[str, str]]:
    """Extract fenced code blocks from markdown text."""
    pattern = r"```(\w*)\n(.*?)```"
    matches = re.findall(pattern, text, re.DOTALL)
    return [{"language": lang or "text", "code": code.strip()} for lang, code in matches]
