# rlm-core

> Unified RLM (Recursive Language Model) orchestration library

A Rust library with Python and Go bindings that provides foundational RLM capabilities for AI coding agents.

## Features

### Core
| Module | Description |
|--------|-------------|
| `context` | Session context, messages, tool outputs, file caching |
| `complexity` | Pattern-based task complexity classification |
| `trajectory` | Observable execution events for streaming UIs |
| `orchestrator` | Core RLM orchestration loop |
| `repl` | Sandboxed Python REPL with async execution |

### Memory & Reasoning
| Module | Description |
|--------|-------------|
| `memory` | Hypergraph knowledge store with tiered lifecycle |
| `reasoning` | Decision trees and trace analysis |
| `epistemic` | Hallucination detection, claim verification |

### LLM Integration
| Module | Description |
|--------|-------------|
| `llm` | Multi-provider client (Anthropic, OpenAI, Google) |
| `llm::SmartRouter` | Intelligent model selection |
| `llm::CostTracker` | Usage and cost monitoring |
| `llm::BatchExecutor` | Concurrent query execution |

### Advanced
| Module | Description |
|--------|-------------|
| `module` | DSPy-style composable AI modules |
| `signature` | Typed signatures with derive macros |
| `proof` | Lean 4 proof automation |
| `sync` | Dual-track sync (informal ↔ formal) |
| `adapters` | Claude Code plugin integration |

## Documentation by Role

### Users and Operators
- [Documentation Portal](../docs/README.md)
- [User Guide](../docs/user-guide/README.md)
- [Troubleshooting](../docs/troubleshooting/README.md)

### Contributors
- [Developer Guide](../docs/developer-guide/README.md)
- [Quality Gates](../docs/developer-guide/quality-gates.md)
- [Execution Plan](../docs/execution-plan/README.md)

### Internals and Design
- [Internals](../docs/internals/README.md)
- [Concepts](../docs/concepts/README.md)
- [Specification Contracts](../docs/spec/)
- [Architecture Decisions](../docs/adr/)

## Installation

### Rust

```toml
[dependencies]
rlm-core = { git = "https://github.com/rand/loop", path = "rlm-core" }
```

### Python

```bash
cd rlm-core/python
uv sync
uv run maturin develop --release

# Or install the wheel
uv run maturin build --release
pip install target/wheels/rlm_core-*.whl
```

### Go

```go
// go.mod
require github.com/rand/loop/rlm-core/go v0.0.0
replace github.com/rand/loop/rlm-core/go => /path/to/loop/rlm-core/go
```

Requires the static library:
```bash
cd rlm-core
cargo build --release --lib --no-default-features --features tokio-runtime
```

## Usage

### Rust

```rust
use rlm_core::{
    SessionContext, PatternClassifier, ActivationDecision,
    SqliteMemoryStore, Node, NodeType, Tier,
};

// Complexity classification
let ctx = SessionContext::new();
ctx.add_user_message("Analyze the auth system");
ctx.cache_file("src/auth.rs", "pub fn login() { ... }");

let classifier = PatternClassifier::default();
let decision = classifier.should_activate("Analyze auth", &ctx);

// Memory store
let store = SqliteMemoryStore::in_memory()?;
let node = Node::new(NodeType::Fact, "Auth uses JWT tokens");
store.add_node(&node)?;

let results = store.search_content("JWT", 10)?;
```

### Python

```python
from rlm_core import (
    SessionContext, PatternClassifier,
    MemoryStore, Node, NodeType, Tier,
    TrajectoryEvent, TrajectoryEventType,
)

# Complexity classification
ctx = SessionContext()
ctx.add_user_message("Find all SQL injection vulnerabilities")

classifier = PatternClassifier()
decision = classifier.should_activate("Find SQL injection", ctx)
print(f"Activate: {decision.should_activate}, Reason: {decision.reason}")

# Memory store
store = MemoryStore.in_memory()
node = Node(NodeType.FACT, "Uses parameterized queries")
store.add_node(node)

# Trajectory events
event = TrajectoryEvent.rlm_start("Analyzing codebase")
print(event.log_line())
```

### Go

```go
package main

import (
    "fmt"
    "github.com/rand/loop/rlm-core/go/rlmcore"
)

func main() {
    rlmcore.Init()
    defer rlmcore.Shutdown()

    // Session context
    ctx := rlmcore.NewSessionContext()
    defer ctx.Free()
    ctx.AddUserMessage("Refactor error handling")

    // Pattern classifier
    classifier := rlmcore.NewPatternClassifier()
    defer classifier.Free()

    decision := classifier.ShouldActivate("Refactor errors", ctx)
    defer decision.Free()

    fmt.Printf("Activate: %v, Reason: %s\n",
        decision.ShouldActivate(), decision.Reason())

    // Memory store
    store := rlmcore.NewMemoryStoreInMemory()
    defer store.Free()

    node := rlmcore.NewNode(rlmcore.NodeTypeFact, "Uses Result<T, E>")
    defer node.Free()
    store.AddNode(node)
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│  ┌──────────────────┐         ┌──────────────────────────┐  │
│  │  Claude Code     │         │     Agentic TUI          │  │
│  │  (Python plugin) │         │     (Go + Bubble Tea)    │  │
│  └────────┬─────────┘         └────────────┬─────────────┘  │
│           │                                │                 │
│           ▼                                ▼                 │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              rlm-core (Rust)                          │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────────┐  │   │
│  │  │ Context │ │ Memory  │ │   LLM   │ │ Trajectory │  │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └────────────┘  │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────────┐  │   │
│  │  │  REPL   │ │Epistemic│ │ Module  │ │   Proof    │  │   │
│  │  └─────────┘ └─────────┘ └─────────┘ └────────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
│           │ PyO3                       │ CGO/FFI            │
│           ▼                            ▼                    │
│  ┌──────────────────┐         ┌──────────────────────┐     │
│  │  Python Bindings │         │    Go Bindings       │     │
│  │  (rlm_core.so)   │         │  (librlm_core.a)     │     │
│  └──────────────────┘         └──────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

## Building

```bash
# Rust library
cargo build --release

# Python bindings
cargo build --release --features python
cd python && uv run maturin develop --release

# Go bindings (static library without Python)
cargo build --release --lib --no-default-features --features tokio-runtime

# Run tests
cargo test
cd go/rlmcore && go test ./...
cd python && uv run pytest
```

## License

MIT
