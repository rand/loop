//! MCP (Model Context Protocol) tool definitions for Claude Code.
//!
//! This module defines the MCP tools that expose rlm-core functionality:
//!
//! - **rlm_execute**: Execute RLM orchestration
//! - **rlm_status**: Get current RLM status
//! - **memory_query**: Query the memory store
//! - **memory_store**: Store data in memory
//! - **trace_visualize**: Export ReasoningTrace artifacts (HTML/DOT/NetworkX/Mermaid)

use crate::error::{Error, Result};
use crate::reasoning::{HtmlConfig, ReasoningTrace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// An MCP tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name (must be unique)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for input parameters
    pub input_schema: Value,
    /// Whether this tool requires confirmation before execution
    pub requires_confirmation: bool,
    /// Category for organization
    pub category: Option<String>,
    /// Example usage
    pub examples: Vec<ToolExample>,
}

impl McpTool {
    /// Create a new MCP tool.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: Value::Object(Default::default()),
            requires_confirmation: false,
            category: None,
            examples: Vec::new(),
        }
    }

    /// Set the input schema.
    pub fn with_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }

    /// Mark as requiring confirmation.
    pub fn requires_confirmation(mut self) -> Self {
        self.requires_confirmation = true;
        self
    }

    /// Set the category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Add an example.
    pub fn with_example(mut self, example: ToolExample) -> Self {
        self.examples.push(example);
        self
    }
}

/// An example of tool usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    /// Example name/title
    pub name: String,
    /// Example input
    pub input: Value,
    /// Expected output description
    pub expected_output: String,
}

impl ToolExample {
    /// Create a new example.
    pub fn new(
        name: impl Into<String>,
        input: Value,
        expected_output: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            input,
            expected_output: expected_output.into(),
        }
    }
}

/// Type alias for tool handler function.
pub type ToolHandler = Arc<dyn Fn(Value) -> Result<Value> + Send + Sync>;

/// Registry of MCP tools.
pub struct McpToolRegistry {
    tools: HashMap<String, (McpTool, ToolHandler)>,
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl McpToolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with default RLM tools.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register default tools
        registry.register_rlm_execute();
        registry.register_rlm_status();
        registry.register_memory_query();
        registry.register_memory_store();
        registry.register_trace_visualize();

        registry
    }

    /// Register a tool with its handler.
    pub fn register(&mut self, tool: McpTool, handler: ToolHandler) {
        self.tools.insert(tool.name.clone(), (tool, handler));
    }

    /// Get a tool definition by name.
    pub fn get_tool(&self, name: &str) -> Option<&McpTool> {
        self.tools.get(name).map(|(tool, _)| tool)
    }

    /// Get all tool definitions.
    pub fn tools(&self) -> Vec<&McpTool> {
        self.tools.values().map(|(tool, _)| tool).collect()
    }

    /// Execute a tool by name.
    pub fn execute(&self, name: &str, input: Value) -> Result<Value> {
        let (_, handler) = self
            .tools
            .get(name)
            .ok_or_else(|| Error::Config(format!("Unknown tool: {}", name)))?;

        handler(input)
    }

    /// Get tool count.
    pub fn count(&self) -> usize {
        self.tools.len()
    }

    /// Get tools by category.
    pub fn tools_by_category(&self, category: &str) -> Vec<&McpTool> {
        self.tools
            .values()
            .filter(|(tool, _)| tool.category.as_deref() == Some(category))
            .map(|(tool, _)| tool)
            .collect()
    }

    /// Export tools as JSON schema for MCP.
    pub fn export_schema(&self) -> Value {
        let tools: Vec<Value> = self
            .tools()
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.input_schema,
                })
            })
            .collect();

        serde_json::json!({
            "tools": tools
        })
    }

    // =========================================================================
    // Default Tool Registrations
    // =========================================================================

    fn register_rlm_execute(&mut self) {
        let tool = McpTool::new(
            "rlm_execute",
            "Execute RLM (Recursive Language Model) orchestration for complex tasks. \
             Automatically analyzes query complexity and activates multi-step reasoning \
             when needed.",
        )
        .with_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The query or task to process"
                },
                "mode": {
                    "type": "string",
                    "enum": ["micro", "fast", "balanced", "thorough"],
                    "description": "Execution mode (default: auto-select based on complexity)"
                },
                "force_activation": {
                    "type": "boolean",
                    "description": "Force RLM activation regardless of complexity analysis",
                    "default": false
                },
                "max_budget_usd": {
                    "type": "number",
                    "description": "Maximum cost budget in USD"
                }
            },
            "required": ["query"]
        }))
        .with_category("rlm")
        .with_example(ToolExample::new(
            "Simple query",
            serde_json::json!({
                "query": "What is the auth flow?"
            }),
            "Analysis of authentication flow with relevant code references",
        ))
        .with_example(ToolExample::new(
            "Thorough analysis",
            serde_json::json!({
                "query": "Find all security vulnerabilities",
                "mode": "thorough",
                "force_activation": true
            }),
            "Comprehensive security audit with findings",
        ));

        let handler: ToolHandler = Arc::new(|input| {
            // This is a placeholder - actual implementation connects to adapter
            Ok(serde_json::json!({
                "status": "pending",
                "message": "RLM execution queued",
                "input": input
            }))
        });

        self.register(tool, handler);
    }

    fn register_rlm_status(&mut self) {
        let tool = McpTool::new(
            "rlm_status",
            "Get the current status of RLM including execution mode, budget state, \
             and memory statistics.",
        )
        .with_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "include_memory_stats": {
                    "type": "boolean",
                    "description": "Include detailed memory statistics",
                    "default": false
                },
                "include_budget_details": {
                    "type": "boolean",
                    "description": "Include detailed budget breakdown",
                    "default": false
                }
            }
        }))
        .with_category("rlm")
        .with_example(ToolExample::new(
            "Basic status",
            serde_json::json!({}),
            "Current mode, execution state, and summary stats",
        ));

        let handler: ToolHandler = Arc::new(|_input| {
            // Placeholder
            Ok(serde_json::json!({
                "mode": "micro",
                "is_executing": false,
                "budget": {
                    "current_cost_usd": 0.0,
                    "max_cost_usd": 1.0
                }
            }))
        });

        self.register(tool, handler);
    }

    fn register_memory_query(&mut self) {
        let tool = McpTool::new(
            "memory_query",
            "Query the RLM memory store for relevant knowledge. Supports text search, \
             type filtering, and semantic similarity.",
        )
        .with_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text query for content search"
                },
                "node_types": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["entity", "fact", "experience", "decision", "snippet"]
                    },
                    "description": "Filter by node types"
                },
                "tiers": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["task", "session", "longterm", "archive"]
                    },
                    "description": "Filter by memory tiers"
                },
                "min_confidence": {
                    "type": "number",
                    "description": "Minimum confidence threshold (0.0-1.0)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results to return",
                    "default": 10
                }
            }
        }))
        .with_category("memory")
        .with_example(ToolExample::new(
            "Search for auth-related facts",
            serde_json::json!({
                "text": "authentication",
                "node_types": ["fact", "entity"],
                "limit": 5
            }),
            "List of relevant memory nodes about authentication",
        ));

        let handler: ToolHandler = Arc::new(|_input| {
            // Placeholder
            Ok(serde_json::json!({
                "nodes": [],
                "total_count": 0
            }))
        });

        self.register(tool, handler);
    }

    fn register_memory_store(&mut self) {
        let tool = McpTool::new(
            "memory_store",
            "Store new knowledge in the RLM memory system. Knowledge is automatically \
             tiered and can be promoted based on access patterns.",
        )
        .with_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The content to store"
                },
                "node_type": {
                    "type": "string",
                    "enum": ["entity", "fact", "experience", "decision", "snippet"],
                    "description": "Type of knowledge node"
                },
                "subtype": {
                    "type": "string",
                    "description": "Optional subtype for finer categorization"
                },
                "confidence": {
                    "type": "number",
                    "description": "Confidence score (0.0-1.0)",
                    "default": 1.0
                },
                "tier": {
                    "type": "string",
                    "enum": ["task", "session", "longterm"],
                    "description": "Initial storage tier",
                    "default": "task"
                },
                "metadata": {
                    "type": "object",
                    "description": "Additional metadata"
                }
            },
            "required": ["content", "node_type"]
        }))
        .with_category("memory")
        .with_example(ToolExample::new(
            "Store a fact",
            serde_json::json!({
                "content": "The API uses JWT tokens for authentication",
                "node_type": "fact",
                "confidence": 0.95
            }),
            "Confirmation with node ID",
        ));

        let handler: ToolHandler = Arc::new(|_input| {
            // Placeholder
            Ok(serde_json::json!({
                "success": true,
                "node_id": "placeholder-id"
            }))
        });

        self.register(tool, handler);
    }

    fn register_trace_visualize(&mut self) {
        let tool = McpTool::new(
            "trace_visualize",
            "Export a serialized ReasoningTrace into visualization artifacts \
             (html, dot, networkx_json, or mermaid).",
        )
        .with_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "trace_json": {
                    "type": "string",
                    "description": "Serialized ReasoningTrace JSON payload"
                },
                "format": {
                    "type": "string",
                    "enum": ["html", "dot", "networkx_json", "mermaid"],
                    "description": "Requested output format",
                    "default": "html"
                },
                "html_preset": {
                    "type": "string",
                    "enum": ["default", "minimal", "presentation"],
                    "description": "Preset used when format=html",
                    "default": "default"
                }
            },
            "required": ["trace_json"]
        }))
        .with_category("reasoning")
        .with_example(ToolExample::new(
            "Export Mermaid trace",
            serde_json::json!({
                "trace_json": "{\"id\":\"...\"}",
                "format": "mermaid"
            }),
            "Mermaid graph with trace metadata header",
        ));

        let handler: ToolHandler = Arc::new(|input| {
            let trace_json = input
                .get("trace_json")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Config("trace_json is required".to_string()))?;

            let trace: ReasoningTrace = serde_json::from_str(trace_json)
                .map_err(|e| Error::Config(format!("Invalid trace_json: {}", e)))?;

            let format = input
                .get("format")
                .and_then(Value::as_str)
                .unwrap_or("html");

            let artifact = match format {
                "html" => {
                    let preset = input
                        .get("html_preset")
                        .and_then(Value::as_str)
                        .unwrap_or("default");
                    let config = match preset {
                        "default" => HtmlConfig::default(),
                        "minimal" => HtmlConfig::minimal(),
                        "presentation" => HtmlConfig::presentation(),
                        other => {
                            return Err(Error::Config(format!(
                                "Unsupported html_preset: {}",
                                other
                            )))
                        }
                    };
                    trace.to_html(config)
                }
                "dot" => trace.to_dot(),
                "networkx_json" => trace.to_networkx_json(),
                "mermaid" => trace.to_mermaid_enhanced(),
                other => {
                    return Err(Error::Config(format!("Unsupported format: {}", other)));
                }
            };

            Ok(serde_json::json!({
                "trace_id": trace.id.to_string(),
                "format": format,
                "node_count": trace.nodes.len(),
                "edge_count": trace.edges.len(),
                "artifact": artifact
            }))
        });

        self.register(tool, handler);
    }
}

/// Input for rlm_execute tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmExecuteInput {
    pub query: String,
    pub mode: Option<String>,
    pub force_activation: Option<bool>,
    pub max_budget_usd: Option<f64>,
}

/// Input for rlm_status tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmStatusInput {
    pub include_memory_stats: Option<bool>,
    pub include_budget_details: Option<bool>,
}

/// Input for memory_query tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryInput {
    pub text: Option<String>,
    pub node_types: Option<Vec<String>>,
    pub tiers: Option<Vec<String>>,
    pub min_confidence: Option<f64>,
    pub limit: Option<usize>,
}

/// Input for memory_store tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreInput {
    pub content: String,
    pub node_type: String,
    pub subtype: Option<String>,
    pub confidence: Option<f64>,
    pub tier: Option<String>,
    pub metadata: Option<HashMap<String, Value>>,
}

/// Input for trace_visualize tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceVisualizeInput {
    pub trace_json: String,
    pub format: Option<String>,
    pub html_preset: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_creation() {
        let tool = McpTool::new("test_tool", "A test tool")
            .with_category("testing")
            .requires_confirmation();

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
        assert_eq!(tool.category, Some("testing".to_string()));
        assert!(tool.requires_confirmation);
    }

    #[test]
    fn test_mcp_tool_with_schema() {
        let tool = McpTool::new("test", "test")
            .with_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                }
            }));

        assert!(tool.input_schema.is_object());
    }

    #[test]
    fn test_registry_default_tools() {
        let registry = McpToolRegistry::with_defaults();

        assert_eq!(registry.count(), 5);
        assert!(registry.get_tool("rlm_execute").is_some());
        assert!(registry.get_tool("rlm_status").is_some());
        assert!(registry.get_tool("memory_query").is_some());
        assert!(registry.get_tool("memory_store").is_some());
        assert!(registry.get_tool("trace_visualize").is_some());
    }

    #[test]
    fn test_registry_tools_by_category() {
        let registry = McpToolRegistry::with_defaults();

        let rlm_tools = registry.tools_by_category("rlm");
        assert_eq!(rlm_tools.len(), 2);

        let memory_tools = registry.tools_by_category("memory");
        assert_eq!(memory_tools.len(), 2);

        let reasoning_tools = registry.tools_by_category("reasoning");
        assert_eq!(reasoning_tools.len(), 1);
    }

    #[test]
    fn test_registry_execute() {
        let registry = McpToolRegistry::with_defaults();

        let result = registry.execute("rlm_status", serde_json::json!({}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_trace_visualize_mermaid_export() {
        let registry = McpToolRegistry::with_defaults();
        let mut trace = ReasoningTrace::new("Visualize this trace", "mcp-session");
        let root = trace.root_goal.clone();
        trace.log_decision(&root, "Pick option", &["A", "B"], 0, "A selected");
        let trace_json = serde_json::to_string(&trace).expect("trace should serialize");

        let result = registry
            .execute(
                "trace_visualize",
                serde_json::json!({
                    "trace_json": trace_json,
                    "format": "mermaid"
                }),
            )
            .expect("trace_visualize should succeed");

        assert_eq!(result.get("format").and_then(Value::as_str), Some("mermaid"));
        let artifact = result
            .get("artifact")
            .and_then(Value::as_str)
            .expect("artifact must be string");
        assert!(artifact.contains("%% ReasoningTrace (enhanced)"));
        assert!(artifact.contains("graph TD"));
    }

    #[test]
    fn test_registry_execute_unknown() {
        let registry = McpToolRegistry::with_defaults();

        let result = registry.execute("unknown_tool", serde_json::json!({}));
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_export_schema() {
        let registry = McpToolRegistry::with_defaults();
        let schema = registry.export_schema();

        assert!(schema.is_object());
        assert!(schema.get("tools").is_some());
    }

    #[test]
    fn test_tool_example() {
        let example = ToolExample::new(
            "Example 1",
            serde_json::json!({"query": "test"}),
            "Expected output",
        );

        assert_eq!(example.name, "Example 1");
        assert_eq!(example.expected_output, "Expected output");
    }
}
