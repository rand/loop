# SPEC-25: Context-as-Variable Enforcement

> Prevent context rot via externalized context variables

**Status**: Draft
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-bw2

---

## Overview

Enforce the context-as-variable pattern where the root LLM receives only the query while full context is stored as Python variables in the REPL. This prevents "context rot" where LLM performance degrades with lengthy context in prompts.

## Background

From Codecrack3 RLM-DSPy:
- Direct API calls fail at 0% accuracy on 132k-token tasks
- RLM with externalized context achieves 80% accuracy
- Token consumption: ~2-3k tokens vs 95k+ for direct
- Context exploration via programmatic access (slicing, regex, recursive calls)

## Requirements

### SPEC-25.01: Context Externalization

Structure for externalized context.

```rust
/// Externalized context for RLM execution
#[derive(Debug, Clone)]
pub struct ExternalizedContext {
    /// Query text (sent to LLM in prompt)
    pub query: String,
    /// Variables available in REPL (NOT sent in prompt)
    pub variables: HashMap<String, ContextVariable>,
    /// Total size of externalized data
    pub total_size_bytes: usize,
}

/// A single context variable
#[derive(Debug, Clone)]
pub struct ContextVariable {
    /// Variable name (Python identifier)
    pub name: String,
    /// Type of context
    pub var_type: ContextVarType,
    /// Size in bytes
    pub size_bytes: usize,
    /// Brief summary for LLM (what this variable contains)
    pub summary: String,
    /// Number of items (for collections)
    pub item_count: Option<usize>,
}

/// Types of context variables
#[derive(Debug, Clone)]
pub enum ContextVarType {
    /// Conversation history: List[Message]
    Conversation,
    /// File contents: Dict[str, str]
    Files,
    /// Tool outputs: List[ToolOutput]
    ToolOutputs,
    /// Working memory: Dict[str, Any]
    WorkingMemory,
    /// Custom context
    Custom(String),
}

impl ExternalizedContext {
    /// Create from SessionContext
    pub fn from_session(ctx: &SessionContext, query: &str) -> Self;

    /// Get variable summaries for prompt
    pub fn variable_summaries(&self) -> String;

    /// Check if context exceeds size limits
    pub fn check_size_limits(&self, config: &SizeConfig) -> Vec<SizeWarning>;
}
```

**Acceptance Criteria**:
- [ ] All context types externalized
- [ ] Summaries generated for each variable
- [ ] Size tracking accurate

### SPEC-25.02: Root Prompt Generation

Generate prompts without full context.

```rust
impl Orchestrator {
    /// Generate root prompt with externalized context
    fn generate_root_prompt(
        &self,
        query: &str,
        external: &ExternalizedContext,
    ) -> String {
        format!(r#"
You have access to the following context variables in the REPL:

{variable_summaries}

To explore the context, use Python code in the REPL. Available helpers:
- peek(var, start, end) - Get slice of collection
- search(var, pattern) - Search for pattern (regex supported)
- summarize(var) - Get LLM summary of variable
- len(var) - Get size of collection

Your task: {query}

Write Python code to explore the context and find the answer.
When done, call SUBMIT({{...}}) with your outputs.
"#,
            variable_summaries = external.variable_summaries(),
            query = query
        )
    }
}
```

**Prompt Rules**:
- Root prompt MUST NOT include full context
- Root prompt MUST include variable summaries
- Root prompt MUST list available helpers
- Root prompt MUST instruct REPL exploration

**Acceptance Criteria**:
- [ ] Prompt contains summaries, not full content
- [ ] Helper functions documented in prompt
- [ ] SUBMIT instruction included

### SPEC-25.03: Variable Access Helpers

REPL helper functions for context access.

```python
# Available in REPL sandbox

def peek(var, start: int = 0, end: int = 10):
    """
    Get slice of a collection.

    Args:
        var: Collection to slice (list, dict values, string)
        start: Start index (default 0)
        end: End index (default 10)

    Returns:
        Sliced content with metadata
    """
    if isinstance(var, list):
        return var[start:end]
    elif isinstance(var, dict):
        keys = list(var.keys())[start:end]
        return {k: var[k] for k in keys}
    elif isinstance(var, str):
        return var[start:end]
    else:
        raise TypeError(f"Cannot peek into {type(var)}")


def search(var, pattern: str, regex: bool = False, max_results: int = 10):
    """
    Search for pattern in context.

    Args:
        var: Context to search (dict of files, list of messages, etc.)
        pattern: Search pattern (string or regex)
        regex: Whether pattern is regex (default False)
        max_results: Maximum results to return (default 10)

    Returns:
        List of matches with location info
    """
    import re
    if regex:
        pat = re.compile(pattern)
        match_fn = lambda s: pat.search(s) is not None
    else:
        match_fn = lambda s: pattern in s

    results = []
    if isinstance(var, dict):
        for key, value in var.items():
            if match_fn(str(value)):
                results.append({"key": key, "preview": str(value)[:200]})
    elif isinstance(var, list):
        for i, item in enumerate(var):
            if match_fn(str(item)):
                results.append({"index": i, "preview": str(item)[:200]})

    return results[:max_results]


def summarize(var, max_tokens: int = 500) -> str:
    """
    Get LLM summary of variable (deferred operation).

    Args:
        var: Variable to summarize
        max_tokens: Maximum tokens in summary

    Returns:
        Summary string (via deferred LLM call)
    """
    # Returns DeferredOperation, resolved by orchestrator
    return _deferred_llm_call(
        f"Summarize the following in {max_tokens} tokens or less:\n{var}"
    )


def find_relevant(var, query: str, top_k: int = 5):
    """
    Find most relevant items for a query.

    Args:
        var: Collection to search
        query: Query string
        top_k: Number of results

    Returns:
        Top k relevant items (via embedding similarity)
    """
    # Returns DeferredOperation, resolved by orchestrator
    return _deferred_embedding_search(var, query, top_k)
```

**Acceptance Criteria**:
- [ ] peek() works for all collection types
- [ ] search() supports regex and literal
- [ ] summarize() returns deferred operation
- [ ] find_relevant() uses embeddings

### SPEC-25.04: Context Size Limits

Size tracking and enforcement.

```rust
/// Configuration for context size limits
#[derive(Debug, Clone)]
pub struct SizeConfig {
    /// Warning threshold per variable (bytes)
    pub warn_threshold: usize,      // Default: 100KB
    /// Error threshold per variable (bytes)
    pub chunk_threshold: usize,     // Default: 1MB
    /// Maximum total externalized size (bytes)
    pub max_total_size: usize,      // Default: 10MB
}

impl Default for SizeConfig {
    fn default() -> Self {
        Self {
            warn_threshold: 100 * 1024,        // 100KB
            chunk_threshold: 1024 * 1024,      // 1MB
            max_total_size: 10 * 1024 * 1024,  // 10MB
        }
    }
}

/// Warning for size limit issues
#[derive(Debug, Clone)]
pub enum SizeWarning {
    /// Variable exceeds warning threshold
    LargeVariable {
        name: String,
        size: usize,
        threshold: usize,
    },
    /// Variable requires chunking
    RequiresChunking {
        name: String,
        size: usize,
        suggested_chunks: usize,
    },
    /// Total size exceeds maximum
    TotalSizeExceeded {
        total: usize,
        max: usize,
    },
}

impl ExternalizedContext {
    /// Check size limits and return warnings
    pub fn check_size_limits(&self, config: &SizeConfig) -> Vec<SizeWarning> {
        let mut warnings = Vec::new();

        for (name, var) in &self.variables {
            if var.size_bytes > config.chunk_threshold {
                warnings.push(SizeWarning::RequiresChunking {
                    name: name.clone(),
                    size: var.size_bytes,
                    suggested_chunks: (var.size_bytes / config.warn_threshold) + 1,
                });
            } else if var.size_bytes > config.warn_threshold {
                warnings.push(SizeWarning::LargeVariable {
                    name: name.clone(),
                    size: var.size_bytes,
                    threshold: config.warn_threshold,
                });
            }
        }

        if self.total_size_bytes > config.max_total_size {
            warnings.push(SizeWarning::TotalSizeExceeded {
                total: self.total_size_bytes,
                max: config.max_total_size,
            });
        }

        warnings
    }

    /// Auto-chunk large variables
    pub fn auto_chunk(&mut self, config: &SizeConfig);
}
```

**Acceptance Criteria**:
- [ ] Warnings generated for large variables
- [ ] Chunking suggested when needed
- [ ] Total size tracked

---

## Performance Impact

| Scenario | Direct API | Externalized | Improvement |
|----------|------------|--------------|-------------|
| 132k tokens | ~95k prompt | ~2-3k prompt | 97% reduction |
| 60k structured | 0% accuracy | 80% accuracy | N/A (enables) |
| 150k+ tokens | Fails | Works | N/A (enables) |

---

## Test Plan

| Test | Description | Spec |
|------|-------------|------|
| `test_externalize_session` | Externalize SessionContext | SPEC-25.01 |
| `test_prompt_no_full_context` | Prompt lacks full context | SPEC-25.02 |
| `test_peek_list` | peek() on list | SPEC-25.03 |
| `test_peek_dict` | peek() on dict | SPEC-25.03 |
| `test_search_literal` | search() literal | SPEC-25.03 |
| `test_search_regex` | search() regex | SPEC-25.03 |
| `test_size_warning` | Large variable warning | SPEC-25.04 |
| `test_chunking` | Auto-chunking | SPEC-25.04 |
| `test_comparison` | With/without externalization | SPEC-25.04 |

---

## References

- [Codecrack3 RLM-DSPy](https://github.com/codecrack3/Recursive-Language-Models-RLM-with-DSpy)
- [RLM Paper](https://arxiv.org/abs/2512.24601) - Prompts as manipulable objects
