# rlm-repl

Sandboxed Python REPL subprocess for rlm-core.

## Overview

This package provides a JSON-RPC based Python REPL that runs as a subprocess,
offering safe code execution with RLM-specific helper functions.

For broader project documentation (user/dev/internals/troubleshooting), see:
- `../../docs/README.md`
- `../../docs/user-guide/learning-paths.md`
- `../../docs/reference/command-reference.md`
- `../../docs/troubleshooting/incident-playbook.md`

## Features

- **Sandboxed Execution**: Uses RestrictedPython for safe code execution
- **JSON-RPC Protocol**: Communication via stdin/stdout
- **RLM Helpers**: Built-in functions for `peek`, `search`, `llm`, `summarize`
- **Deferred Operations**: Async LLM calls with placeholder resolution

## Usage

```bash
# Run as subprocess
python -m rlm_repl

# Script entrypoint (equivalent)
rlm-repl
```

The REPL communicates via JSON-RPC over stdin/stdout. See the protocol module
for message formats.

## Helper Functions

Available in the REPL sandbox:

- `peek(data, start, end)` - View slice of data
- `search(data, pattern, regex=False)` - Search for patterns
- `llm(prompt, context=None)` - Make LLM call (deferred)
- `llm_batch(prompts, contexts=None, max_parallel=5, model=None, max_tokens=1024)` - Parallel LLM calls (deferred)
- `llm_query_batched(...)` - Compatibility alias for `llm_batch` (deprecated)
- `summarize(data, max_tokens=500)` - Summarize data (deferred)
- `find_relevant(data, query, top_k=5)` - Semantic search (deferred)

## License

MIT
