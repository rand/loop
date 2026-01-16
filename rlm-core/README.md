# rlm-core

Unified RLM (Recursive Language Model) orchestration library supporting both Claude Code plugins and agentic TUIs.

## Features

- **Context Management**: Session context, messages, and tool outputs
- **Memory System**: Hypergraph-based knowledge store with tiered lifecycle
- **LLM Client**: Multi-provider support with smart routing and cost tracking
- **Trajectory**: Observable execution events for streaming and analysis
- **Complexity Detection**: Pattern-based activation decisions

## Installation

### Python

```bash
pip install rlm-core
```

### Rust

```toml
[dependencies]
rlm-core = "0.1"
```

## Usage

### Python

```python
from rlm_core import SessionContext, Message, PatternClassifier

# Create a session context
ctx = SessionContext()
ctx.add_message(Message.user("Analyze the auth system"))

# Check if RLM should activate
classifier = PatternClassifier()
decision = classifier.should_activate("Analyze the auth system", ctx)
if decision.should_activate:
    print(f"RLM activated: {decision.reason}")
```

### Rust

```rust
use rlm_core::{SessionContext, PatternClassifier};

let classifier = PatternClassifier::default();
let ctx = SessionContext::new();

let decision = classifier.should_activate("Analyze the auth system", &ctx);
if decision.should_activate {
    println!("RLM activated: {}", decision.reason);
}
```

## License

MIT
