# Loop

> Unified RLM (Recursive Language Model) orchestration monorepo

Loop provides the core infrastructure for building AI coding agents that handle arbitrarily large contexts through recursive decomposition, persistent memory, and intelligent orchestration.

## Repository Structure

```
loop/
├── rlm-core/           # Core Rust library with Python and Go bindings
│   ├── src/            # Rust source
│   ├── go/             # Go bindings (CGO)
│   └── python/         # Python bindings (PyO3)
├── rlm-core-derive/    # Proc macros for typed signatures
└── docs/               # Design documents and specs
```

## What is RLM?

RLM (Recursive Language Model) enables AI agents to handle arbitrarily large contexts by decomposing complex tasks into smaller sub-queries. Instead of processing 500K tokens at once, RLM lets agents:

- **Externalize** context as manipulable variables in a Python REPL
- **Analyze** task complexity to select appropriate strategies
- **Decompose** large contexts and process in parallel
- **Recurse** with sub-queries for verification and exploration
- **Synthesize** results into coherent final answers
- **Remember** facts and experiences across sessions

## rlm-core Features

### Core Capabilities
- **Context Management**: Session state, messages, tool outputs, file caching
- **Pattern Classifier**: Complexity-based activation decisions
- **REPL Integration**: Sandboxed Python execution with async support
- **Trajectory Events**: Observable execution stream for UI/logging

### Memory & Reasoning
- **Hypergraph Memory**: SQLite-backed knowledge store with tiered lifecycle
- **Reasoning Traces**: Decision trees linked to outcomes
- **Epistemic Verification**: Hallucination detection and claim verification

### LLM Integration
- **Multi-Provider**: Anthropic, OpenAI, Google, local models
- **Smart Router**: Intelligent model selection based on task complexity
- **Cost Tracking**: Per-call and session-level cost monitoring
- **Batched Queries**: Concurrent execution with configurable parallelism

### Advanced Features
- **DSPy-Style Modules**: Composable AI modules with typed signatures
- **Proof Automation**: Lean 4 integration for formal verification
- **Dual-Track Sync**: Keep informal specs and formal proofs aligned
- **Claude Code Adapter**: Plugin integration with hooks and MCP tools

## Quick Start

### Prerequisites

- **Rust 1.75+**: `rustup update stable`
- **Python 3.11+**: For Python bindings
- **Go 1.22+**: For Go bindings
- **uv**: `curl -LsSf https://astral.sh/uv/install.sh | sh`

### Building

```bash
# Clone the repository
git clone https://github.com/rand/loop.git
cd loop

# Build the Rust library
cd rlm-core
cargo build --release

# Build Python bindings
cd python
uv sync
uv run maturin develop --release

# Build Go bindings (requires static library)
cd ../go/rlmcore
go test ./...
```

### Rust Usage

```rust
use rlm_core::{SessionContext, PatternClassifier, ActivationDecision};

let ctx = SessionContext::new();
ctx.add_user_message("Analyze the auth system across all modules");

let classifier = PatternClassifier::default();
let decision = classifier.should_activate("Analyze the auth system", &ctx);

if decision.should_activate {
    println!("RLM mode: {}", decision.reason);
}
```

### Python Usage

```python
from rlm_core import SessionContext, PatternClassifier

ctx = SessionContext()
ctx.add_user_message("Find all security vulnerabilities")

classifier = PatternClassifier()
decision = classifier.should_activate("Find all security vulnerabilities", ctx)

if decision.should_activate:
    print(f"RLM activated: {decision.reason}")
```

### Go Usage

```go
import "github.com/rand/loop/rlm-core/go/rlmcore"

ctx := rlmcore.NewSessionContext()
defer ctx.Free()

ctx.AddUserMessage("Refactor the authentication module")

classifier := rlmcore.NewPatternClassifier()
defer classifier.Free()

decision := classifier.ShouldActivate("Refactor the authentication module", ctx)
defer decision.Free()

if decision.ShouldActivate() {
    fmt.Println("RLM activated:", decision.Reason())
}
```

## Integrations

rlm-core powers these projects:

| Project | Description | Branch |
|---------|-------------|--------|
| [recurse](https://github.com/rand/recurse) | Agentic TUI with Bubble Tea | `rlm-core-migration` |
| [rlm-claude-code](https://github.com/rand/rlm-claude-code) | Claude Code plugin | `rlm-core-migration` |

## Documentation

- [Unified Library Design](docs/unified-rlm-library-design.md) - Architecture overview
- [Migration Spec: recurse](docs/migration-spec-recurse.md) - Go integration guide
- [Migration Spec: rlm-claude-code](docs/migration-spec-rlm-claude-code.md) - Python integration guide
- [Implementation Roadmap](docs/implementation-roadmap.md) - Development phases

## License

MIT
