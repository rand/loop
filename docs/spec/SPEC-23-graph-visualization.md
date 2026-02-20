# SPEC-23: Graph Visualization for ReasoningTrace

> Interactive debugging visualization for reasoning traces

**Status**: Partially implemented (core exports + TUI/MCP integration endpoints are implemented; deferred CLI/advanced HTML controls are tracked in `loop-azq`)
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-wve

---

## Overview

Add interactive graph visualization for ReasoningTrace to enable debugging of complex reasoning chains. Based on Codecrack3's NetworkX visualization approach.

## Implementation Snapshot (2026-02-20)

| Section | Status | Runtime Evidence |
|---|---|---|
| SPEC-23.01 Graph export formats | Implemented (NetworkX JSON, DOT, HTML, enhanced Mermaid) | `rlm-core/src/reasoning/visualize.rs` |
| SPEC-23.02 NetworkX schema | Implemented (runtime node-link schema) | `NetworkXGraph` types and export tests in `rlm-core/src/reasoning/visualize.rs` |
| SPEC-23.03 HTML visualization | Partially implemented | `ReasoningTrace::to_html` + `test_html_export` in `rlm-core/src/reasoning/visualize.rs` |
| SPEC-23.04 Integration points | Partially implemented (TUI + MCP, CLI deferred) | `TUIAdapter::render_trace_panel` and `trace_visualize` in `rlm-core/src/adapters/` |

## Requirements

### SPEC-23.01: Graph Export Formats

Multiple export formats for different use cases.

```rust
impl ReasoningTrace {
    /// Export to NetworkX-compatible JSON
    pub fn to_networkx_json(&self) -> String {
        serde_json::to_string(&self.to_graph_data()).unwrap()
    }

    /// Export to interactive HTML (D3.js-based)
    pub fn to_html(&self, config: &HtmlConfig) -> String;

    /// Export to DOT format (Graphviz)
    pub fn to_dot(&self) -> String;

    /// Export to Mermaid (enhanced from existing)
    pub fn to_mermaid_enhanced(&self) -> String;

    /// Internal graph data structure
    fn to_graph_data(&self) -> GraphData;
}

pub struct HtmlConfig {
    /// Page title
    pub title: String,
    /// Include cost annotations
    pub show_costs: bool,
    /// Include timing annotations
    pub show_timing: bool,
    /// Expand REPL history by default
    pub expand_repl: bool,
    /// Color scheme
    pub theme: Theme,
}

pub enum Theme {
    Light,
    Dark,
    HighContrast,
}
```

**Acceptance Criteria**:
- [ ] to_networkx_json() produces valid JSON
- [ ] to_html() produces self-contained HTML
- [ ] to_dot() produces valid Graphviz DOT
- [ ] All formats render correctly

### SPEC-23.02: NetworkX JSON Schema

Schema for graph interchange.

```json
{
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "properties": {
        "nodes": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "type": {
                        "type": "string",
                        "enum": ["Goal", "Decision", "Option", "Action", "Outcome"]
                    },
                    "content": { "type": "string" },
                    "metadata": { "type": "object" },
                    "timing_ms": { "type": "integer" },
                    "cost_usd": { "type": "number" },
                    "depth": { "type": "integer" },
                    "repl_history": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "code": { "type": "string" },
                                "output": { "type": "string" },
                                "error": { "type": "string" }
                            }
                        }
                    }
                },
                "required": ["id", "type", "content"]
            }
        },
        "edges": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "target": { "type": "string" },
                    "label": {
                        "type": "string",
                        "enum": ["Spawns", "Chooses", "Rejects", "Proves", "Supports", "Contradicts", "Depends"]
                    },
                    "weight": { "type": "number" }
                },
                "required": ["source", "target", "label"]
            }
        },
        "metadata": {
            "type": "object",
            "properties": {
                "trace_id": { "type": "string" },
                "root_goal": { "type": "string" },
                "session_id": { "type": "string" },
                "total_cost_usd": { "type": "number" },
                "total_time_ms": { "type": "integer" },
                "git_commit": { "type": "string" },
                "git_branch": { "type": "string" }
            }
        }
    },
    "required": ["nodes", "edges", "metadata"]
}
```

**Acceptance Criteria**:
- [ ] JSON validates against schema
- [ ] Importable into NetworkX Python library
- [ ] All node/edge types represented

### SPEC-23.03: HTML Visualization Features

Interactive HTML visualization requirements.

**Canvas Features**:
- Zoomable (mouse wheel)
- Pannable (drag)
- Fit-to-view button
- Reset view button

**Node Rendering**:
- Color coded by type:
  - Goal: Blue (#3498db)
  - Decision: Purple (#9b59b6)
  - Option: Gray (#95a5a6)
  - Action: Green (#2ecc71)
  - Outcome: Orange (#e67e22)
  - Error: Red (#e74c3c)
- Size proportional to cost/importance
- Icon indicating type

**Node Details Panel** (on click):
- Full content text
- Metadata table
- Timing information
- Cost breakdown
- REPL history (expandable)
- Copy button for content

**Edge Rendering**:
- Arrows showing direction
- Label on hover
- Color by type:
  - Spawns: Gray
  - Chooses: Green
  - Rejects: Red (dashed)
  - Proves: Blue (bold)

**Annotations**:
- Cost badges on nodes (optional)
- Timing badges on edges (optional)
- Error highlighting (red glow)

**Export**:
- PNG export button
- SVG export button
- JSON data download

**Acceptance Criteria**:
- [ ] All canvas features work
- [ ] Node details panel complete
- [ ] Export functions work
- [ ] Responsive design (mobile-friendly)

### SPEC-23.04: Integration Points

Integration with existing systems.

```rust
// CLI integration
impl Cli {
    /// Visualize a trace
    #[command]
    pub fn trace_visualize(
        &self,
        trace_id: &str,
        #[arg(long, default_value = "html")]
        format: OutputFormat,
        #[arg(long)]
        output: Option<PathBuf>,
    ) -> Result<()>;
}

// TUI adapter integration
impl TUIAdapter {
    /// Render trace visualization panel
    pub fn render_trace_panel(&self, trace: &ReasoningTrace) -> String;
}

// Claude Code adapter integration
impl ClaudeCodeAdapter {
    /// MCP resource for trace HTML
    #[mcp_resource(uri = "rlm://trace/{trace_id}/html")]
    pub fn get_trace_html(&self, trace_id: &str) -> String;

    /// MCP tool for trace visualization
    #[mcp_tool]
    pub fn visualize_trace(&self, trace_id: &str) -> TraceVisualization;
}
```

**Acceptance Criteria**:
- [ ] CLI command works
- [ ] TUI panel renders
- [ ] MCP resource accessible

---

## Implementation Notes

### D3.js Template

The HTML visualization should use D3.js for graph rendering:

```html
<!DOCTYPE html>
<html>
<head>
    <title>{{title}}</title>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <style>
        /* Styles for nodes, edges, panels */
    </style>
</head>
<body>
    <div id="graph"></div>
    <div id="details-panel"></div>
    <script>
        const data = {{json_data}};
        // D3.js force-directed graph
    </script>
</body>
</html>
```

### File Locations

| Component | Location |
|-----------|----------|
| Graph export | `rlm-core/src/reasoning/visualize.rs` |
| Mermaid base export | `rlm-core/src/reasoning/trace.rs` |
| TUI integration surface | `rlm-core/src/adapters/tui/adapter.rs` |
| Claude Code MCP integration surface | `rlm-core/src/adapters/claude_code/mcp.rs` |

---

## Test Plan

| Test | Description | Spec |
|------|-------------|------|
| `test_networkx_json_valid` | JSON validates schema | SPEC-23.02 |
| `test_html_renders` | HTML is valid | SPEC-23.03 |
| `test_dot_valid` | DOT is valid Graphviz | SPEC-23.01 |
| `test_node_colors` | Correct color by type | SPEC-23.03 |
| `test_mermaid_enhanced_export` | Enhanced Mermaid includes metadata and valid graph body | SPEC-23.01 |
| `test_trace_visualize_mermaid_export` | MCP visualization endpoint exports Mermaid artifact | SPEC-23.04 |
| `test_render_trace_panel_contains_mermaid` | TUI integration surface renders deterministic Mermaid panel payload | SPEC-23.04 |

---

## References

- [Codecrack3 NetworkX visualization](https://github.com/codecrack3/Recursive-Language-Models-RLM-with-DSpy)
- [D3.js Force-Directed Graph](https://observablehq.com/@d3/force-directed-graph)
- Existing ReasoningTrace: `src/reasoning/trace.rs`
