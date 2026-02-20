//! Graph visualization exports for ReasoningTrace.
//!
//! Provides multiple export formats for reasoning traces:
//! - NetworkX-compatible JSON (SPEC-23.02)
//! - DOT/Graphviz format (SPEC-23.01)
//! - Interactive HTML with D3.js (SPEC-23.03)
//!
//! # Example
//!
//! ```rust,ignore
//! use rlm_core::reasoning::ReasoningTrace;
//!
//! let trace = ReasoningTrace::new("Implement feature", "session-1");
//!
//! // Export to NetworkX JSON for Python analysis
//! let networkx_json = trace.to_networkx_json();
//!
//! // Export to Graphviz DOT
//! let dot = trace.to_dot();
//!
//! // Export to interactive HTML
//! let html = trace.to_html(HtmlConfig::default());
//! ```

use crate::reasoning::trace::ReasoningTrace;
use crate::reasoning::types::{DecisionNodeType, TraceEdgeLabel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Theme presets for HTML visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlTheme {
    /// Dark background with light text.
    Dark,
    /// Light background with dark text.
    Light,
    /// High-contrast palette for accessibility.
    HighContrast,
}

/// Configuration for HTML visualization export.
#[derive(Debug, Clone)]
pub struct HtmlConfig {
    /// Width of the visualization in pixels.
    pub width: u32,
    /// Height of the visualization in pixels.
    pub height: u32,
    /// Title for the page.
    pub title: String,
    /// Whether to include pan/zoom controls.
    pub enable_pan_zoom: bool,
    /// Whether to show node labels.
    pub show_labels: bool,
    /// Whether to show edge labels.
    pub show_edge_labels: bool,
    /// Whether to animate transitions.
    pub animate: bool,
    /// Render a node details side panel.
    pub show_details_panel: bool,
    /// Include export buttons (PNG/SVG/JSON).
    pub show_export_controls: bool,
    /// Show cost badges when metadata is present.
    pub show_cost_badges: bool,
    /// Show timing badges when metadata is present.
    pub show_timing_badges: bool,
    /// Expand REPL history blocks by default.
    pub expand_repl_history: bool,
    /// Apply fit-to-view after layout settles.
    pub fit_to_view_on_load: bool,
    /// Theme preset.
    pub theme: HtmlTheme,
    /// Node colors by type.
    pub node_colors: HashMap<DecisionNodeType, String>,
    /// Custom CSS to inject.
    pub custom_css: Option<String>,
}

impl Default for HtmlConfig {
    fn default() -> Self {
        let mut node_colors = HashMap::new();
        node_colors.insert(DecisionNodeType::Goal, "#90EE90".to_string()); // Light green
        node_colors.insert(DecisionNodeType::Decision, "#FFD700".to_string()); // Gold
        node_colors.insert(DecisionNodeType::Option, "#87CEEB".to_string()); // Sky blue
        node_colors.insert(DecisionNodeType::Action, "#DDA0DD".to_string()); // Plum
        node_colors.insert(DecisionNodeType::Outcome, "#98FB98".to_string()); // Pale green
        node_colors.insert(DecisionNodeType::Observation, "#F0E68C".to_string()); // Khaki

        Self {
            width: 1200,
            height: 800,
            title: "Reasoning Trace Visualization".to_string(),
            enable_pan_zoom: true,
            show_labels: true,
            show_edge_labels: true,
            animate: true,
            show_details_panel: true,
            show_export_controls: true,
            show_cost_badges: true,
            show_timing_badges: true,
            expand_repl_history: false,
            fit_to_view_on_load: true,
            theme: HtmlTheme::Dark,
            node_colors,
            custom_css: None,
        }
    }
}

impl HtmlConfig {
    /// Create a minimal configuration.
    pub fn minimal() -> Self {
        Self {
            width: 800,
            height: 600,
            title: "Reasoning Trace".to_string(),
            enable_pan_zoom: false,
            show_labels: true,
            show_edge_labels: false,
            animate: false,
            show_details_panel: false,
            show_export_controls: false,
            show_cost_badges: false,
            show_timing_badges: false,
            expand_repl_history: false,
            fit_to_view_on_load: false,
            theme: HtmlTheme::Light,
            ..Default::default()
        }
    }

    /// Create a presentation-focused configuration.
    pub fn presentation() -> Self {
        Self {
            width: 1600,
            height: 900,
            title: "Reasoning Trace".to_string(),
            enable_pan_zoom: true,
            show_labels: true,
            show_edge_labels: true,
            animate: true,
            show_details_panel: true,
            show_export_controls: true,
            show_cost_badges: true,
            show_timing_badges: true,
            expand_repl_history: true,
            fit_to_view_on_load: true,
            theme: HtmlTheme::Dark,
            ..Default::default()
        }
    }

    /// Set custom width.
    pub fn with_width(mut self, width: u32) -> Self {
        self.width = width;
        self
    }

    /// Set custom height.
    pub fn with_height(mut self, height: u32) -> Self {
        self.height = height;
        self
    }

    /// Set title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set visualization theme.
    pub fn with_theme(mut self, theme: HtmlTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Toggle node details side panel.
    pub fn with_details_panel(mut self, enabled: bool) -> Self {
        self.show_details_panel = enabled;
        self
    }

    /// Toggle export controls.
    pub fn with_export_controls(mut self, enabled: bool) -> Self {
        self.show_export_controls = enabled;
        self
    }

    /// Toggle fit-to-view on initial render.
    pub fn with_fit_to_view(mut self, enabled: bool) -> Self {
        self.fit_to_view_on_load = enabled;
        self
    }

    /// Expand or collapse REPL history by default.
    pub fn with_expand_repl_history(mut self, expand: bool) -> Self {
        self.expand_repl_history = expand;
        self
    }

    /// Set custom CSS.
    pub fn with_css(mut self, css: impl Into<String>) -> Self {
        self.custom_css = Some(css.into());
        self
    }
}

/// DOT export configuration.
#[derive(Debug, Clone)]
pub struct DotConfig {
    /// Graph direction: "TB" (top-bottom), "LR" (left-right), etc.
    pub rankdir: String,
    /// Whether to use filled node style.
    pub filled_nodes: bool,
    /// Font name for labels.
    pub font_name: String,
    /// Font size for labels.
    pub font_size: u32,
    /// Node colors by type.
    pub node_colors: HashMap<DecisionNodeType, String>,
}

impl Default for DotConfig {
    fn default() -> Self {
        let mut node_colors = HashMap::new();
        node_colors.insert(DecisionNodeType::Goal, "#90EE90".to_string());
        node_colors.insert(DecisionNodeType::Decision, "#FFD700".to_string());
        node_colors.insert(DecisionNodeType::Option, "#87CEEB".to_string());
        node_colors.insert(DecisionNodeType::Action, "#DDA0DD".to_string());
        node_colors.insert(DecisionNodeType::Outcome, "#98FB98".to_string());
        node_colors.insert(DecisionNodeType::Observation, "#F0E68C".to_string());

        Self {
            rankdir: "TB".to_string(),
            filled_nodes: true,
            font_name: "Helvetica".to_string(),
            font_size: 12,
            node_colors,
        }
    }
}

impl DotConfig {
    /// Create a left-to-right layout.
    pub fn left_to_right() -> Self {
        Self {
            rankdir: "LR".to_string(),
            ..Default::default()
        }
    }
}

/// NetworkX-compatible JSON format (node-link data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkXGraph {
    /// Whether the graph is directed.
    pub directed: bool,
    /// Whether the graph supports multiple edges between nodes.
    pub multigraph: bool,
    /// Graph-level attributes.
    pub graph: NetworkXGraphAttrs,
    /// List of nodes.
    pub nodes: Vec<NetworkXNode>,
    /// List of edges (links).
    pub links: Vec<NetworkXLink>,
}

/// Graph-level attributes for NetworkX format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkXGraphAttrs {
    /// Trace ID.
    pub trace_id: String,
    /// Session ID.
    pub session_id: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Git commit if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
    /// Git branch if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

/// A node in NetworkX format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkXNode {
    /// Node ID (UUID string).
    pub id: String,
    /// Node type (goal, decision, option, etc.).
    pub node_type: String,
    /// Node content/label.
    pub content: String,
    /// Confidence score.
    pub confidence: f64,
    /// Optional reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Whether this is the root node.
    pub is_root: bool,
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// An edge (link) in NetworkX format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkXLink {
    /// Source node ID.
    pub source: String,
    /// Target node ID.
    pub target: String,
    /// Edge label/type.
    pub label: String,
    /// Edge weight.
    pub weight: f64,
    /// Creation timestamp.
    pub created_at: String,
    /// Additional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ReasoningTrace {
    /// Export to NetworkX-compatible JSON format.
    ///
    /// This produces a node-link format compatible with:
    /// - `networkx.node_link_graph()` in Python
    /// - Standard graph interchange formats
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let trace = ReasoningTrace::new("Goal", "session-1");
    /// let json = trace.to_networkx_json();
    ///
    /// // In Python:
    /// // import networkx as nx
    /// // import json
    /// // G = nx.node_link_graph(json.loads(json_str))
    /// ```
    pub fn to_networkx_json(&self) -> String {
        let graph = self.to_networkx_graph();
        serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string())
    }

    /// Convert to NetworkX graph structure.
    pub fn to_networkx_graph(&self) -> NetworkXGraph {
        let nodes: Vec<NetworkXNode> = self
            .nodes
            .iter()
            .map(|n| NetworkXNode {
                id: n.id.0.to_string(),
                node_type: n.node_type.to_string(),
                content: n.content.clone(),
                confidence: n.confidence,
                reason: n.reason.clone(),
                created_at: n.created_at.to_rfc3339(),
                is_root: n.id == self.root_goal,
                metadata: n
                    .metadata
                    .as_ref()
                    .map(|m| serde_json::to_value(m).ok())
                    .flatten(),
            })
            .collect();

        let links: Vec<NetworkXLink> = self
            .edges
            .iter()
            .map(|e| NetworkXLink {
                source: e.from.0.to_string(),
                target: e.to.0.to_string(),
                label: e.label.to_string(),
                weight: e.weight,
                created_at: e.created_at.to_rfc3339(),
                metadata: e
                    .metadata
                    .as_ref()
                    .map(|m| serde_json::to_value(m).ok())
                    .flatten(),
            })
            .collect();

        NetworkXGraph {
            directed: true,
            multigraph: false,
            graph: NetworkXGraphAttrs {
                trace_id: self.id.to_string(),
                session_id: self.session_id.clone(),
                created_at: self.created_at.to_rfc3339(),
                git_commit: self.git_commit.clone(),
                git_branch: self.git_branch.clone(),
            },
            nodes,
            links,
        }
    }

    /// Export to DOT/Graphviz format.
    ///
    /// Produces a DOT language representation that can be rendered with
    /// Graphviz tools like `dot`, `neato`, or `fdp`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let trace = ReasoningTrace::new("Goal", "session-1");
    /// let dot = trace.to_dot();
    ///
    /// // Save and render:
    /// // dot -Tpng trace.dot -o trace.png
    /// // dot -Tsvg trace.dot -o trace.svg
    /// ```
    pub fn to_dot(&self) -> String {
        self.to_dot_with_config(&DotConfig::default())
    }

    /// Export to enhanced Mermaid format with trace metadata comments.
    ///
    /// This builds on `ReasoningTrace::to_mermaid()` by prepending trace-level
    /// metadata so generated diagrams remain self-describing in artifacts.
    pub fn to_mermaid_enhanced(&self) -> String {
        let mut mermaid = String::new();
        mermaid.push_str("%% ReasoningTrace (enhanced)\n");
        mermaid.push_str(&format!("%% trace_id: {}\n", self.id));
        mermaid.push_str(&format!("%% session_id: {}\n", self.session_id));
        mermaid.push_str(&format!("%% node_count: {}\n", self.nodes.len()));
        mermaid.push_str(&format!("%% edge_count: {}\n\n", self.edges.len()));
        mermaid.push_str(&self.to_mermaid());
        mermaid
    }

    /// Export to DOT format with custom configuration.
    pub fn to_dot_with_config(&self, config: &DotConfig) -> String {
        let mut dot = String::new();

        // Graph header
        dot.push_str("digraph ReasoningTrace {\n");
        dot.push_str(&format!("    rankdir={};\n", config.rankdir));
        dot.push_str(&format!(
            "    node [fontname=\"{}\", fontsize={}",
            config.font_name, config.font_size
        ));
        if config.filled_nodes {
            dot.push_str(", style=filled");
        }
        dot.push_str("];\n");
        dot.push_str(&format!(
            "    edge [fontname=\"{}\", fontsize={}];\n",
            config.font_name,
            config.font_size - 2
        ));
        dot.push('\n');

        // Graph metadata as comment
        dot.push_str(&format!("    // Trace ID: {}\n", self.id));
        dot.push_str(&format!("    // Session: {}\n", self.session_id));
        if let Some(ref commit) = self.git_commit {
            dot.push_str(&format!("    // Git commit: {}\n", commit));
        }
        dot.push('\n');

        // Nodes
        for node in &self.nodes {
            let node_id = format!("n{}", node.id.0.as_simple());
            let label = escape_dot_string(&truncate_string(&node.content, 40));
            let shape = node_type_to_dot_shape(node.node_type);
            let color = config
                .node_colors
                .get(&node.node_type)
                .map(|s| s.as_str())
                .unwrap_or("#FFFFFF");

            // Mark root node specially
            let extra = if node.id == self.root_goal {
                ", penwidth=3"
            } else {
                ""
            };

            dot.push_str(&format!(
                "    {} [label=\"{}\", shape={}, fillcolor=\"{}\"{}];\n",
                node_id, label, shape, color, extra
            ));
        }

        dot.push('\n');

        // Edges
        for edge in &self.edges {
            let from_id = format!("n{}", edge.from.0.as_simple());
            let to_id = format!("n{}", edge.to.0.as_simple());
            let label = edge.label.to_string();
            let style = edge_label_to_dot_style(edge.label);

            dot.push_str(&format!(
                "    {} -> {} [label=\"{}\", {}];\n",
                from_id, to_id, label, style
            ));
        }

        dot.push_str("}\n");
        dot
    }

    /// Export to interactive HTML with D3.js visualization.
    ///
    /// Produces a self-contained HTML file with an interactive force-directed
    /// graph visualization using D3.js. Features include:
    /// - Pan and zoom
    /// - Node hover tooltips
    /// - Click to expand/collapse
    /// - Edge label display
    /// - Color-coded node types
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let trace = ReasoningTrace::new("Goal", "session-1");
    /// let html = trace.to_html(HtmlConfig::default());
    ///
    /// std::fs::write("trace.html", html)?;
    /// // Open trace.html in a browser
    /// ```
    pub fn to_html(&self, config: HtmlConfig) -> String {
        let networkx_json = self.to_networkx_json();
        generate_html(&networkx_json, &config)
    }
}

// Helper functions

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn escape_dot_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn node_type_to_dot_shape(node_type: DecisionNodeType) -> &'static str {
    match node_type {
        DecisionNodeType::Goal => "doubleoctagon",
        DecisionNodeType::Decision => "diamond",
        DecisionNodeType::Option => "box",
        DecisionNodeType::Action => "parallelogram",
        DecisionNodeType::Outcome => "ellipse",
        DecisionNodeType::Observation => "note",
    }
}

fn edge_label_to_dot_style(label: TraceEdgeLabel) -> &'static str {
    match label {
        TraceEdgeLabel::Chooses => "color=\"#228B22\", penwidth=2",
        TraceEdgeLabel::Rejects => "color=\"#DC143C\", style=dashed",
        TraceEdgeLabel::Spawns => "color=\"#4169E1\", penwidth=2",
        TraceEdgeLabel::Implements => "color=\"#9400D3\", penwidth=1.5",
        TraceEdgeLabel::Produces => "color=\"#FF8C00\"",
        TraceEdgeLabel::LeadsTo => "color=\"#808080\", style=dotted",
        TraceEdgeLabel::References => "color=\"#A9A9A9\", style=dashed",
        TraceEdgeLabel::Requires => "color=\"#FF4500\", style=bold",
        TraceEdgeLabel::Invalidates => "color=\"#8B0000\", style=bold",
        TraceEdgeLabel::Considers => "color=\"#4682B4\"",
    }
}

fn generate_html(graph_json: &str, config: &HtmlConfig) -> String {
    let node_colors_json = serde_json::to_string(
        &config
            .node_colors
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect::<HashMap<String, String>>(),
    )
    .unwrap_or_else(|_| "{}".to_string());

    let custom_css = config.custom_css.as_deref().unwrap_or("");
    let (bg_color, text_color, panel_bg, panel_border, panel_shadow, tooltip_bg, tooltip_text) =
        match config.theme {
            HtmlTheme::Dark => (
                "#0f172a",
                "#e2e8f0",
                "rgba(15, 23, 42, 0.88)",
                "#334155",
                "rgba(15, 23, 42, 0.45)",
                "rgba(2, 6, 23, 0.94)",
                "#e2e8f0",
            ),
            HtmlTheme::Light => (
                "#f8fafc",
                "#0f172a",
                "rgba(255, 255, 255, 0.94)",
                "#cbd5e1",
                "rgba(148, 163, 184, 0.35)",
                "rgba(255, 255, 255, 0.98)",
                "#0f172a",
            ),
            HtmlTheme::HighContrast => (
                "#000000",
                "#ffffff",
                "rgba(0, 0, 0, 0.96)",
                "#ffffff",
                "rgba(255, 255, 255, 0.40)",
                "rgba(0, 0, 0, 0.98)",
                "#ffffff",
            ),
        };
    let details_panel_display = if config.show_details_panel {
        "block"
    } else {
        "none"
    };
    let export_controls_display = if config.show_export_controls {
        "inline-flex"
    } else {
        "none"
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: "IBM Plex Sans", "Helvetica Neue", "Segoe UI", sans-serif;
            background: {bg_color};
            color: {text_color};
            overflow: hidden;
        }}

        #container {{
            width: 100vw;
            height: 100vh;
            position: relative;
        }}

        svg {{
            width: 100%;
            height: 100%;
        }}

        .node {{
            cursor: pointer;
            transition: transform 0.2s ease;
        }}

        .node:hover {{
            transform: scale(1.1);
        }}

        .node circle {{
            stroke: #fff;
            stroke-width: 2px;
        }}

        .node.root circle {{
            stroke-width: 4px;
            stroke: #f59e0b;
        }}

        .node text {{
            font-size: 11px;
            fill: {text_color};
            text-anchor: middle;
            pointer-events: none;
        }}

        .link {{
            fill: none;
            stroke-opacity: 0.6;
        }}

        .link.chooses {{
            stroke: #228B22;
            stroke-width: 3px;
        }}

        .link.rejects {{
            stroke: #DC143C;
            stroke-dasharray: 5,5;
        }}

        .link.spawns {{
            stroke: #4169E1;
            stroke-width: 2px;
        }}

        .link-label {{
            font-size: 10px;
            fill: {text_color};
            opacity: 0.75;
            pointer-events: none;
        }}

        .tooltip {{
            position: absolute;
            background: {tooltip_bg};
            color: {tooltip_text};
            border: 1px solid {panel_border};
            border-radius: 8px;
            padding: 12px;
            font-size: 13px;
            max-width: 350px;
            pointer-events: none;
            opacity: 0;
            transition: opacity 0.2s;
            z-index: 1000;
            box-shadow: 0 12px 26px -14px {panel_shadow};
        }}

        .tooltip.visible {{
            opacity: 1;
        }}

        .tooltip h3 {{
            margin-bottom: 8px;
            color: inherit;
            font-size: 14px;
        }}

        .tooltip p {{
            margin: 4px 0;
            color: inherit;
        }}

        .tooltip .type {{
            display: inline-block;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 11px;
            text-transform: uppercase;
            margin-bottom: 8px;
        }}

        .legend {{
            position: absolute;
            top: 20px;
            right: 20px;
            background: {panel_bg};
            border: 1px solid {panel_border};
            border-radius: 8px;
            padding: 16px;
            box-shadow: 0 14px 30px -20px {panel_shadow};
        }}

        .legend h4 {{
            margin-bottom: 12px;
            color: inherit;
        }}

        .legend-item {{
            display: flex;
            align-items: center;
            margin: 6px 0;
        }}

        .legend-color {{
            width: 16px;
            height: 16px;
            border-radius: 50%;
            margin-right: 10px;
            border: 2px solid #fff;
        }}

        .controls {{
            position: absolute;
            top: 20px;
            left: 20px;
            background: {panel_bg};
            border: 1px solid {panel_border};
            border-radius: 8px;
            padding: 12px;
            box-shadow: 0 14px 30px -20px {panel_shadow};
        }}

        .controls button {{
            display: block;
            width: 100%;
            margin: 4px 0;
            padding: 8px 16px;
            background: transparent;
            border: 1px solid {panel_border};
            border-radius: 4px;
            color: {text_color};
            cursor: pointer;
            transition: background 0.2s, transform 0.15s;
        }}

        .controls button:hover {{
            background: rgba(148, 163, 184, 0.2);
            transform: translateY(-1px);
        }}

        .controls .export-group {{
            display: {export_controls_display};
            width: 100%;
            margin-top: 6px;
            flex-direction: column;
            gap: 4px;
        }}

        .stats {{
            position: absolute;
            bottom: 20px;
            left: 20px;
            background: {panel_bg};
            border: 1px solid {panel_border};
            border-radius: 8px;
            padding: 12px;
            font-size: 12px;
            box-shadow: 0 14px 30px -20px {panel_shadow};
        }}

        .stats span {{
            display: block;
            margin: 4px 0;
        }}

        .details-panel {{
            display: {details_panel_display};
            position: absolute;
            right: 20px;
            bottom: 20px;
            width: min(420px, calc(100vw - 40px));
            max-height: min(48vh, 440px);
            overflow: auto;
            background: {panel_bg};
            border: 1px solid {panel_border};
            border-radius: 8px;
            padding: 12px;
            box-shadow: 0 14px 30px -20px {panel_shadow};
        }}

        .details-panel h4 {{
            margin-bottom: 8px;
        }}

        .details-empty {{
            opacity: 0.75;
            font-size: 13px;
        }}

        .details-table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 10px;
            font-size: 12px;
        }}

        .details-table td {{
            border-bottom: 1px solid {panel_border};
            padding: 5px 0;
            vertical-align: top;
        }}

        .details-table td:first-child {{
            width: 36%;
            font-weight: 600;
            opacity: 0.85;
        }}

        .details-panel pre {{
            white-space: pre-wrap;
            word-break: break-word;
            margin-top: 8px;
            background: rgba(148, 163, 184, 0.16);
            border-radius: 6px;
            padding: 8px;
            font-size: 12px;
        }}

        .details-panel details {{
            margin-top: 8px;
        }}

        .details-panel summary {{
            cursor: pointer;
            font-weight: 600;
        }}

        .copy-content {{
            margin-top: 10px;
            width: 100%;
            padding: 7px 10px;
            border-radius: 6px;
            border: 1px solid {panel_border};
            background: transparent;
            color: {text_color};
            cursor: pointer;
        }}

        .copy-content:hover {{
            background: rgba(148, 163, 184, 0.2);
        }}

        @media (max-width: 900px) {{
            .legend {{
                right: 12px;
                top: 12px;
                max-width: 42vw;
            }}

            .controls {{
                left: 12px;
                top: 12px;
                max-width: 42vw;
            }}

            .stats {{
                left: 12px;
                bottom: 12px;
                max-width: 42vw;
            }}

            .details-panel {{
                right: 12px;
                bottom: 12px;
                width: 46vw;
            }}
        }}

        {custom_css}
    </style>
</head>
<body>
    <div id="container">
        <svg></svg>
        <div class="tooltip" id="tooltip"></div>

        <div class="controls">
            <button onclick="resetZoom()">Reset View</button>
            <button onclick="fitToView()">Fit to View</button>
            <button onclick="toggleLabels()">Toggle Labels</button>
            <button onclick="toggleEdgeLabels()">Toggle Edge Labels</button>
            <div class="export-group">
                <button onclick="exportPng()">Export PNG</button>
                <button onclick="exportSvg()">Export SVG</button>
                <button onclick="downloadJson()">Download JSON</button>
            </div>
        </div>

        <div class="legend">
            <h4>Node Types</h4>
            <div id="legend-items"></div>
        </div>

        <div class="stats" id="stats"></div>

        <aside class="details-panel" id="details-panel">
            <h4>Node Details</h4>
            <div id="details-content" class="details-empty">Click a node to inspect full content, metadata, and REPL history.</div>
            <button id="copy-content" class="copy-content" type="button">Copy Content</button>
        </aside>
    </div>

    <script>
        // Graph data
        const graphData = {graph_json};
        const nodeColors = {node_colors_json};
        const config = {{
            width: {width},
            height: {height},
            showLabels: {show_labels},
            showEdgeLabels: {show_edge_labels},
            animate: {animate},
            enablePanZoom: {enable_pan_zoom},
            showCostBadges: {show_cost_badges},
            showTimingBadges: {show_timing_badges},
            expandReplHistory: {expand_repl_history},
            fitToViewOnLoad: {fit_to_view_on_load},
            showDetailsPanel: {show_details_panel},
            showExportControls: {show_export_controls}
        }};

        // State
        let showLabels = config.showLabels;
        let showEdgeLabels = config.showEdgeLabels;
        let selectedNodeContent = "";

        // Setup SVG
        const svg = d3.select("svg");
        const container = svg.append("g");

        // Zoom behavior
        const zoom = d3.zoom()
            .scaleExtent([0.1, 4])
            .on("zoom", (event) => {{
                container.attr("transform", event.transform);
            }});

        if (config.enablePanZoom) {{
            svg.call(zoom);
        }}

        // Create arrow markers
        svg.append("defs").selectAll("marker")
            .data(["arrow"])
            .join("marker")
            .attr("id", d => d)
            .attr("viewBox", "0 -5 10 10")
            .attr("refX", 20)
            .attr("refY", 0)
            .attr("markerWidth", 6)
            .attr("markerHeight", 6)
            .attr("orient", "auto")
            .append("path")
            .attr("fill", "#888")
            .attr("d", "M0,-5L10,0L0,5");

        // Process data
        const nodes = graphData.nodes.map(d => ({{...d}}));
        const links = graphData.links.map(d => ({{
            ...d,
            source: nodes.find(n => n.id === d.source),
            target: nodes.find(n => n.id === d.target)
        }}));

        // Force simulation
        const simulation = d3.forceSimulation(nodes)
            .force("link", d3.forceLink(links).id(d => d.id).distance(100))
            .force("charge", d3.forceManyBody().strength(-400))
            .force("center", d3.forceCenter(config.width / 2, config.height / 2))
            .force("collision", d3.forceCollide().radius(40));

        // Draw links
        const link = container.append("g")
            .attr("class", "links")
            .selectAll("path")
            .data(links)
            .join("path")
            .attr("class", d => `link ${{d.label}}`)
            .attr("stroke", d => getLinkColor(d.label))
            .attr("stroke-width", d => getLinkWidth(d.label))
            .attr("stroke-dasharray", d => getLinkDash(d.label))
            .attr("marker-end", "url(#arrow)");

        // Draw link labels
        const linkLabel = container.append("g")
            .attr("class", "link-labels")
            .selectAll("text")
            .data(links)
            .join("text")
            .attr("class", "link-label")
            .text(d => d.label)
            .style("opacity", showEdgeLabels ? 1 : 0);

        // Draw nodes
        const node = container.append("g")
            .attr("class", "nodes")
            .selectAll("g")
            .data(nodes)
            .join("g")
            .attr("class", d => `node ${{d.is_root ? 'root' : ''}}`)
            .call(d3.drag()
                .on("start", dragstarted)
                .on("drag", dragged)
                .on("end", dragended));

        node.append("circle")
            .attr("r", d => d.is_root ? 25 : 20)
            .attr("fill", d => nodeColors[d.node_type] || "#ccc");

        // Node labels
        const nodeLabel = node.append("text")
            .attr("dy", 35)
            .text(d => truncate(d.content, 20))
            .style("opacity", showLabels ? 1 : 0);

        // Tooltip
        const tooltip = d3.select("#tooltip");

        node.on("mouseenter", (event, d) => {{
            const html = `
                <span class="type" style="background: ${{nodeColors[d.node_type] || '#ccc'}}">${{d.node_type}}</span>
                <h3>${{escapeHtml(d.content)}}</h3>
                ${{d.reason ? `<p><strong>Reason:</strong> ${{escapeHtml(d.reason)}}</p>` : ''}}
                <p><strong>Confidence:</strong> ${{(d.confidence * 100).toFixed(0)}}%</p>
                ${{config.showCostBadges && d.metadata && d.metadata.cost_usd !== undefined ? `<p><strong>Cost:</strong> $${{Number(d.metadata.cost_usd).toFixed(4)}}</p>` : ''}}
                ${{config.showTimingBadges && d.metadata && d.metadata.timing_ms !== undefined ? `<p><strong>Timing:</strong> ${{d.metadata.timing_ms}} ms</p>` : ''}}
                <p><strong>Created:</strong> ${{new Date(d.created_at).toLocaleString()}}</p>
            `;
            tooltip.html(html)
                .style("left", (event.pageX + 15) + "px")
                .style("top", (event.pageY - 10) + "px")
                .classed("visible", true);
        }})
        .on("mouseleave", () => {{
            tooltip.classed("visible", false);
        }})
        .on("click", (_, d) => {{
            if (config.showDetailsPanel) {{
                renderDetails(d);
            }}
        }});

        // Simulation tick
        simulation.on("tick", () => {{
            link.attr("d", linkArc);

            linkLabel
                .attr("x", d => (d.source.x + d.target.x) / 2)
                .attr("y", d => (d.source.y + d.target.y) / 2);

            node.attr("transform", d => `translate(${{d.x}},${{d.y}})`);
        }});

        // Legend
        const legendItems = d3.select("#legend-items");
        Object.entries(nodeColors).forEach(([type, color]) => {{
            const item = legendItems.append("div").attr("class", "legend-item");
            item.append("div")
                .attr("class", "legend-color")
                .style("background", color);
            item.append("span").text(type);
        }});

        // Stats
        const stats = d3.select("#stats");
        stats.html(`
            <span><strong>Nodes:</strong> ${{nodes.length}}</span>
            <span><strong>Edges:</strong> ${{links.length}}</span>
            <span><strong>Session:</strong> ${{graphData.graph.session_id}}</span>
        `);

        const copyButton = document.getElementById("copy-content");
        if (copyButton) {{
            copyButton.addEventListener("click", async () => {{
                if (!selectedNodeContent) {{
                    return;
                }}
                try {{
                    await navigator.clipboard.writeText(selectedNodeContent);
                    copyButton.textContent = "Copied";
                    setTimeout(() => {{
                        copyButton.textContent = "Copy Content";
                    }}, 1200);
                }} catch (_) {{
                    copyButton.textContent = "Copy Failed";
                    setTimeout(() => {{
                        copyButton.textContent = "Copy Content";
                    }}, 1400);
                }}
            }});
        }}

        // Helper functions
        function linkArc(d) {{
            const dx = d.target.x - d.source.x;
            const dy = d.target.y - d.source.y;
            const dr = Math.sqrt(dx * dx + dy * dy) * 1.5;
            return `M${{d.source.x}},${{d.source.y}}A${{dr}},${{dr}} 0 0,1 ${{d.target.x}},${{d.target.y}}`;
        }}

        function getLinkColor(label) {{
            const colors = {{
                'chooses': '#228B22',
                'rejects': '#DC143C',
                'spawns': '#4169E1',
                'implements': '#9400D3',
                'produces': '#FF8C00',
                'leads_to': '#808080',
                'references': '#A9A9A9',
                'requires': '#FF4500',
                'invalidates': '#8B0000',
                'considers': '#4682B4'
            }};
            return colors[label] || '#888';
        }}

        function getLinkWidth(label) {{
            if (label === 'chooses' || label === 'spawns') return 3;
            if (label === 'rejects') return 1.5;
            return 2;
        }}

        function getLinkDash(label) {{
            if (label === 'rejects' || label === 'references') return '5,5';
            if (label === 'leads_to') return '2,2';
            return null;
        }}

        function dragstarted(event, d) {{
            if (!event.active) simulation.alphaTarget(0.3).restart();
            d.fx = d.x;
            d.fy = d.y;
        }}

        function dragged(event, d) {{
            d.fx = event.x;
            d.fy = event.y;
        }}

        function dragended(event, d) {{
            if (!event.active) simulation.alphaTarget(0);
            d.fx = null;
            d.fy = null;
        }}

        function truncate(str, len) {{
            return str.length > len ? str.slice(0, len) + '...' : str;
        }}

        function escapeHtml(str) {{
            return str.replace(/&/g, '&amp;')
                      .replace(/</g, '&lt;')
                      .replace(/>/g, '&gt;')
                      .replace(/"/g, '&quot;');
        }}

        // Controls
        function resetZoom() {{
            svg.transition().duration(750).call(
                zoom.transform,
                d3.zoomIdentity
            );
        }}

        function toggleLabels() {{
            showLabels = !showLabels;
            nodeLabel.style("opacity", showLabels ? 1 : 0);
        }}

        function toggleEdgeLabels() {{
            showEdgeLabels = !showEdgeLabels;
            linkLabel.style("opacity", showEdgeLabels ? 1 : 0);
        }}

        function fitToView() {{
            const bounds = container.node().getBBox();
            const fullWidth = svg.node().clientWidth;
            const fullHeight = svg.node().clientHeight;
            if (!bounds.width || !bounds.height) {{
                return;
            }}

            const widthScale = fullWidth / bounds.width;
            const heightScale = fullHeight / bounds.height;
            const scale = Math.max(0.1, Math.min(4, Math.min(widthScale, heightScale) * 0.86));
            const translateX = fullWidth / 2 - scale * (bounds.x + bounds.width / 2);
            const translateY = fullHeight / 2 - scale * (bounds.y + bounds.height / 2);

            svg.transition().duration(550).call(
                zoom.transform,
                d3.zoomIdentity.translate(translateX, translateY).scale(scale)
            );
        }}

        function renderDetails(node) {{
            const detailsRoot = document.getElementById("details-content");
            if (!detailsRoot) {{
                return;
            }}

            selectedNodeContent = node.content || "";
            const metadata = node.metadata && typeof node.metadata === "object" ? node.metadata : null;
            const metadataRows = metadata
                ? Object.entries(metadata)
                    .filter(([k]) => k !== "repl_history")
                    .map(([k, v]) => `<tr><td>${{escapeHtml(k)}}</td><td>${{escapeHtml(JSON.stringify(v))}}</td></tr>`)
                    .join("")
                : "";

            const replEntries = metadata && Array.isArray(metadata.repl_history) ? metadata.repl_history : [];
            const replOpen = config.expandReplHistory ? "open" : "";
            const replHtml = replEntries.length === 0
                ? "<p>No REPL history captured.</p>"
                : replEntries.map((entry, index) => `
                    <details ${{replOpen}}>
                        <summary>REPL step ${{index + 1}}</summary>
                        <pre><strong>code</strong>\n${{escapeHtml(entry.code || "")}}</pre>
                        <pre><strong>output</strong>\n${{escapeHtml(entry.output || "")}}</pre>
                        ${{entry.error ? `<pre><strong>error</strong>\n${{escapeHtml(entry.error)}}</pre>` : ""}}
                    </details>
                `).join("");

            detailsRoot.innerHTML = `
                <p><strong>Type:</strong> ${{escapeHtml(node.node_type || "unknown")}}</p>
                <p><strong>Confidence:</strong> ${{(Number(node.confidence || 0) * 100).toFixed(0)}}%</p>
                <pre>${{escapeHtml(node.content || "")}}</pre>
                ${{metadataRows ? `<table class="details-table">${{metadataRows}}</table>` : "<p>No metadata fields.</p>"}}
                <h4 style="margin-top:10px;">REPL History</h4>
                ${{replHtml}}
            `;
        }}

        function exportSvg() {{
            const serializer = new XMLSerializer();
            const source = serializer.serializeToString(svg.node());
            const blob = new Blob([source], {{ type: "image/svg+xml;charset=utf-8" }});
            downloadBlob(blob, `${{graphData.graph.trace_id || "trace"}}.svg`);
        }}

        function exportPng() {{
            const serializer = new XMLSerializer();
            const source = serializer.serializeToString(svg.node());
            const svgBlob = new Blob([source], {{ type: "image/svg+xml;charset=utf-8" }});
            const url = URL.createObjectURL(svgBlob);
            const image = new Image();
            image.onload = () => {{
                const canvas = document.createElement("canvas");
                canvas.width = svg.node().clientWidth;
                canvas.height = svg.node().clientHeight;
                const ctx = canvas.getContext("2d");
                ctx.fillStyle = getComputedStyle(document.body).backgroundColor;
                ctx.fillRect(0, 0, canvas.width, canvas.height);
                ctx.drawImage(image, 0, 0);
                canvas.toBlob((blob) => {{
                    if (blob) {{
                        downloadBlob(blob, `${{graphData.graph.trace_id || "trace"}}.png`);
                    }}
                    URL.revokeObjectURL(url);
                }});
            }};
            image.src = url;
        }}

        function downloadJson() {{
            const blob = new Blob([JSON.stringify(graphData, null, 2)], {{ type: "application/json" }});
            downloadBlob(blob, `${{graphData.graph.trace_id || "trace"}}.json`);
        }}

        function downloadBlob(blob, fileName) {{
            const url = URL.createObjectURL(blob);
            const anchor = document.createElement("a");
            anchor.href = url;
            anchor.download = fileName;
            anchor.click();
            URL.revokeObjectURL(url);
        }}

        if (config.fitToViewOnLoad) {{
            setTimeout(() => fitToView(), 400);
        }}
    </script>
</body>
</html>"##,
        title = config.title,
        graph_json = graph_json,
        node_colors_json = node_colors_json,
        width = config.width,
        height = config.height,
        bg_color = bg_color,
        text_color = text_color,
        panel_bg = panel_bg,
        panel_border = panel_border,
        panel_shadow = panel_shadow,
        tooltip_bg = tooltip_bg,
        tooltip_text = tooltip_text,
        details_panel_display = details_panel_display,
        export_controls_display = export_controls_display,
        show_labels = if config.show_labels { "true" } else { "false" },
        show_edge_labels = if config.show_edge_labels {
            "true"
        } else {
            "false"
        },
        animate = if config.animate { "true" } else { "false" },
        enable_pan_zoom = if config.enable_pan_zoom {
            "true"
        } else {
            "false"
        },
        show_cost_badges = if config.show_cost_badges {
            "true"
        } else {
            "false"
        },
        show_timing_badges = if config.show_timing_badges {
            "true"
        } else {
            "false"
        },
        expand_repl_history = if config.expand_repl_history {
            "true"
        } else {
            "false"
        },
        fit_to_view_on_load = if config.fit_to_view_on_load {
            "true"
        } else {
            "false"
        },
        show_details_panel = if config.show_details_panel {
            "true"
        } else {
            "false"
        },
        show_export_controls = if config.show_export_controls {
            "true"
        } else {
            "false"
        },
        custom_css = custom_css,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning::ReasoningTrace;

    #[test]
    fn test_networkx_json_export() {
        let mut trace = ReasoningTrace::new("Test goal", "session-1");
        let root = trace.root_goal.clone();
        trace.log_decision(&root, "Choose option", &["A", "B"], 0, "Best choice");

        let json = trace.to_networkx_json();

        // Parse and verify structure
        let graph: NetworkXGraph = serde_json::from_str(&json).expect("Valid JSON");
        assert!(graph.directed);
        assert!(!graph.multigraph);
        assert_eq!(graph.nodes.len(), 4); // goal + decision + 2 options
        assert!(!graph.links.is_empty());

        // Check root node
        let root_node = graph.nodes.iter().find(|n| n.is_root).unwrap();
        assert_eq!(root_node.node_type, "goal");
    }

    #[test]
    fn test_dot_export() {
        let mut trace = ReasoningTrace::new("Build API", "session-2");
        let root = trace.root_goal.clone();
        trace.log_decision(&root, "Framework", &["Axum", "Actix"], 0, "Performance");

        let dot = trace.to_dot();

        assert!(dot.starts_with("digraph ReasoningTrace"));
        assert!(dot.contains("rankdir=TB"));
        assert!(dot.contains("shape=doubleoctagon")); // Goal shape
        assert!(dot.contains("shape=diamond")); // Decision shape
        assert!(dot.contains("->")); // Edges
        assert!(dot.contains("chooses"));
    }

    #[test]
    fn test_mermaid_enhanced_export() {
        let mut trace = ReasoningTrace::new("Visualize trace", "session-mermaid");
        let root = trace.root_goal.clone();
        trace.log_decision(&root, "Choose approach", &["A", "B"], 0, "A is simpler");

        let mermaid = trace.to_mermaid_enhanced();

        assert!(mermaid.contains("%% ReasoningTrace (enhanced)"));
        assert!(mermaid.contains("%% trace_id:"));
        assert!(mermaid.contains("%% session_id: session-mermaid"));
        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("classDef goal"));
    }

    #[test]
    fn test_dot_config() {
        let trace = ReasoningTrace::new("Test", "session-3");
        let config = DotConfig::left_to_right();
        let dot = trace.to_dot_with_config(&config);

        assert!(dot.contains("rankdir=LR"));
    }

    #[test]
    fn test_html_export() {
        let trace = ReasoningTrace::new("Feature", "session-4");
        let html = trace.to_html(HtmlConfig::default());

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("d3.v7.min.js"));
        assert!(html.contains("Reasoning Trace Visualization"));
        assert!(html.contains("const graphData"));
        assert!(html.contains("Fit to View"));
        assert!(html.contains("Export PNG"));
        assert!(html.contains("details-panel"));
    }

    #[test]
    fn test_html_config() {
        let config = HtmlConfig::minimal();
        assert_eq!(config.width, 800);
        assert!(!config.enable_pan_zoom);
        assert!(!config.show_edge_labels);
        assert!(!config.show_details_panel);
        assert!(!config.show_export_controls);

        let config = HtmlConfig::presentation();
        assert_eq!(config.width, 1600);
        assert!(config.enable_pan_zoom);
        assert!(config.show_details_panel);
        assert!(config.show_export_controls);
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a long string", 10), "this is...");
    }

    #[test]
    fn test_escape_dot_string() {
        assert_eq!(escape_dot_string("hello"), "hello");
        assert_eq!(escape_dot_string("say \"hello\""), "say \\\"hello\\\"");
        assert_eq!(escape_dot_string("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_html_config_builder() {
        let config = HtmlConfig::default()
            .with_width(1000)
            .with_height(700)
            .with_title("My Trace")
            .with_theme(HtmlTheme::HighContrast)
            .with_details_panel(false)
            .with_export_controls(false)
            .with_fit_to_view(false)
            .with_expand_repl_history(true)
            .with_css(".custom { color: red; }");

        assert_eq!(config.width, 1000);
        assert_eq!(config.height, 700);
        assert_eq!(config.title, "My Trace");
        assert_eq!(config.theme, HtmlTheme::HighContrast);
        assert!(!config.show_details_panel);
        assert!(!config.show_export_controls);
        assert!(!config.fit_to_view_on_load);
        assert!(config.expand_repl_history);
        assert!(config.custom_css.is_some());
    }
}
