//! Context externalization to prevent context rot.
//!
//! This module implements the "context-as-variable" pattern from Codecrack3 RLM-DSPy.
//! The root LLM receives only the query and variable summaries, while full context
//! is stored as Python variables accessible via REPL helpers.
//!
//! # Benefits
//!
//! - Prevents performance degradation from lengthy context in prompts
//! - Enables lazy loading of context on demand
//! - Supports large context that wouldn't fit in prompt
//! - Provides structured access patterns (peek, search, summarize)
//!
//! # SPEC-25: Context Externalization
//!
//! - SPEC-25.01: Context variable types and externalization
//! - SPEC-25.02: Root prompt generation without full context
//! - SPEC-25.03: Variable access helpers for REPL
//! - SPEC-25.04: Size tracking and limits

use super::types::SessionContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Size thresholds for context variables (SPEC-25.04).
pub const WARN_SIZE_BYTES: usize = 100 * 1024; // 100 KB
pub const REQUIRE_CHUNKING_BYTES: usize = 1024 * 1024; // 1 MB

/// Type of context variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextVarType {
    /// Conversation history (List[Message]).
    Conversation,
    /// Cached file contents (Dict[str, str]).
    Files,
    /// Tool execution outputs (List[ToolOutput]).
    ToolOutputs,
    /// Working memory (Dict[str, Any]).
    WorkingMemory,
    /// Custom variable type.
    Custom(String),
}

impl std::fmt::Display for ContextVarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conversation => write!(f, "conversation"),
            Self::Files => write!(f, "files"),
            Self::ToolOutputs => write!(f, "tool_outputs"),
            Self::WorkingMemory => write!(f, "working_memory"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// A context variable stored in the REPL namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextVariable {
    /// Variable name in REPL namespace.
    pub name: String,
    /// Type of the variable.
    pub var_type: ContextVarType,
    /// Size in bytes (for tracking limits).
    pub size_bytes: usize,
    /// Brief summary for the LLM (NOT the full content).
    pub summary: String,
    /// Whether this variable exceeds the warning threshold.
    pub size_warning: bool,
    /// Whether this variable requires chunking.
    pub requires_chunking: bool,
    /// Number of items (messages, files, etc.).
    pub item_count: usize,
}

impl ContextVariable {
    /// Create a new context variable.
    pub fn new(
        name: impl Into<String>,
        var_type: ContextVarType,
        size_bytes: usize,
        item_count: usize,
    ) -> Self {
        let name = name.into();
        let summary = Self::generate_summary(&var_type, item_count, size_bytes);
        Self {
            name,
            var_type,
            size_bytes,
            summary,
            size_warning: size_bytes > WARN_SIZE_BYTES,
            requires_chunking: size_bytes > REQUIRE_CHUNKING_BYTES,
            item_count,
        }
    }

    fn generate_summary(var_type: &ContextVarType, count: usize, size: usize) -> String {
        let size_str = if size < 1024 {
            format!("{} bytes", size)
        } else if size < 1024 * 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        };

        match var_type {
            ContextVarType::Conversation => {
                format!("{} messages (~{})", count, size_str)
            }
            ContextVarType::Files => {
                format!("{} files (~{})", count, size_str)
            }
            ContextVarType::ToolOutputs => {
                format!("{} tool outputs (~{})", count, size_str)
            }
            ContextVarType::WorkingMemory => {
                format!("{} entries (~{})", count, size_str)
            }
            ContextVarType::Custom(name) => {
                format!("{}: {} items (~{})", name, count, size_str)
            }
        }
    }
}

/// Externalized context with query and variable summaries (SPEC-25.01).
///
/// The LLM receives only the query and variable summaries.
/// Full context is stored as REPL variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalizedContext {
    /// The user's query (sent to LLM).
    pub query: String,
    /// Variables available in REPL (NOT sent in full to LLM).
    pub variables: HashMap<String, ContextVariable>,
    /// Total size of all externalized context.
    pub total_size_bytes: usize,
    /// Any warnings about context size.
    pub warnings: Vec<String>,
}

impl ExternalizedContext {
    /// Create externalized context from a session context.
    pub fn from_session(ctx: &SessionContext, query: impl Into<String>) -> Self {
        Self::from_session_with_config(ctx, query, &ExternalizationConfig::default())
    }

    /// Create externalized context with custom configuration.
    pub fn from_session_with_config(
        ctx: &SessionContext,
        query: impl Into<String>,
        config: &ExternalizationConfig,
    ) -> Self {
        let mut variables = HashMap::new();
        let mut warnings = Vec::new();
        let mut total_size = 0usize;

        // Externalize conversation
        if !ctx.messages.is_empty() && config.externalize_conversation {
            let size = ctx
                .messages
                .iter()
                .map(|m| m.content.len() + 50) // +50 for role, metadata overhead
                .sum();
            let var = ContextVariable::new(
                "conversation",
                ContextVarType::Conversation,
                size,
                ctx.messages.len(),
            );
            if var.size_warning {
                warnings.push(format!(
                    "conversation exceeds {}KB ({} bytes)",
                    WARN_SIZE_BYTES / 1024,
                    size
                ));
            }
            total_size += size;
            variables.insert("conversation".to_string(), var);
        }

        // Externalize files
        if !ctx.files.is_empty() && config.externalize_files {
            let size: usize = ctx.files.values().map(|c| c.len()).sum();
            let var =
                ContextVariable::new("files", ContextVarType::Files, size, ctx.files.len());
            if var.size_warning {
                warnings.push(format!(
                    "files exceed {}KB ({} bytes)",
                    WARN_SIZE_BYTES / 1024,
                    size
                ));
            }
            if var.requires_chunking {
                warnings.push(format!(
                    "files exceed {}MB - chunking required",
                    REQUIRE_CHUNKING_BYTES / (1024 * 1024)
                ));
            }
            total_size += size;
            variables.insert("files".to_string(), var);
        }

        // Externalize tool outputs
        if !ctx.tool_outputs.is_empty() && config.externalize_tool_outputs {
            let size: usize = ctx
                .tool_outputs
                .iter()
                .map(|o| o.content.len() + o.tool_name.len() + 50)
                .sum();
            let var = ContextVariable::new(
                "tool_outputs",
                ContextVarType::ToolOutputs,
                size,
                ctx.tool_outputs.len(),
            );
            if var.size_warning {
                warnings.push(format!(
                    "tool_outputs exceed {}KB ({} bytes)",
                    WARN_SIZE_BYTES / 1024,
                    size
                ));
            }
            total_size += size;
            variables.insert("tool_outputs".to_string(), var);
        }

        // Externalize working memory
        if !ctx.working_memory.is_empty() && config.externalize_working_memory {
            let size: usize = ctx
                .working_memory
                .iter()
                .map(|(k, v)| k.len() + v.to_string().len())
                .sum();
            let var = ContextVariable::new(
                "working_memory",
                ContextVarType::WorkingMemory,
                size,
                ctx.working_memory.len(),
            );
            total_size += size;
            variables.insert("working_memory".to_string(), var);
        }

        Self {
            query: query.into(),
            variables,
            total_size_bytes: total_size,
            warnings,
        }
    }

    /// Generate the root prompt for the LLM (SPEC-25.02).
    ///
    /// The prompt includes:
    /// - The user's query
    /// - Variable summaries (not full content)
    /// - Instructions for using REPL helpers
    pub fn root_prompt(&self) -> String {
        self.root_prompt_with_config(&RootPromptConfig::default())
    }

    /// Generate root prompt with custom configuration.
    pub fn root_prompt_with_config(&self, config: &RootPromptConfig) -> String {
        let mut prompt = String::new();

        // Query section
        prompt.push_str(&format!("## Query\n\n{}\n\n", self.query));

        // Context variables section
        if !self.variables.is_empty() {
            prompt.push_str("## Available Context Variables\n\n");
            prompt.push_str(
                "The following context is available as Python variables in the REPL.\n",
            );
            prompt.push_str("Use the helper functions to access them efficiently.\n\n");

            for (name, var) in &self.variables {
                prompt.push_str(&format!("- **{}**: {}\n", name, var.summary));
            }
            prompt.push('\n');
        }

        // Helper functions documentation
        if config.include_helper_docs && !self.variables.is_empty() {
            prompt.push_str("## Context Access Helpers\n\n");
            prompt.push_str("```python\n");
            prompt.push_str("# Slice messages (start/end are indices)\n");
            prompt.push_str("peek(conversation, start=0, end=10)\n\n");
            prompt.push_str("# Search in files by regex pattern\n");
            prompt.push_str("search(files, pattern=\"def.*auth\")\n\n");
            prompt.push_str("# Summarize a tool output\n");
            prompt.push_str("summarize(tool_outputs[-1])\n\n");
            prompt.push_str("# Get length of any context variable\n");
            prompt.push_str("len(conversation)\n");
            prompt.push_str("```\n\n");
        }

        // Warnings
        if config.include_warnings && !self.warnings.is_empty() {
            prompt.push_str("## Warnings\n\n");
            for warning in &self.warnings {
                prompt.push_str(&format!("- ⚠️ {}\n", warning));
            }
            prompt.push('\n');
        }

        // Instructions
        if config.include_instructions {
            prompt.push_str("## Instructions\n\n");
            prompt.push_str(
                "DO NOT ask me to paste the full context. Instead, use the REPL helpers above ",
            );
            prompt.push_str(
                "to access exactly what you need. This keeps the conversation efficient.\n",
            );
        }

        prompt
    }

    /// Get REPL setup code to initialize context variables.
    ///
    /// This code should be executed in the REPL before the main task.
    pub fn repl_setup_code(&self, ctx: &SessionContext) -> String {
        let mut code = String::new();
        code.push_str("# Context variable setup\n");
        code.push_str("from rlm_helpers import peek, search, summarize\n\n");

        // Set up conversation
        if self.variables.contains_key("conversation") {
            code.push_str("conversation = [\n");
            for msg in &ctx.messages {
                let role = format!("{}", msg.role);
                let content = msg.content.replace('\\', "\\\\").replace('"', "\\\"");
                // Truncate very long messages in setup
                let content = if content.len() > 1000 {
                    format!("{}...[truncated]", &content[..1000])
                } else {
                    content
                };
                code.push_str(&format!(
                    "    {{\"role\": \"{}\", \"content\": \"{}\"}},\n",
                    role, content
                ));
            }
            code.push_str("]\n\n");
        }

        // Set up files
        if self.variables.contains_key("files") {
            code.push_str("files = {\n");
            for (path, content) in &ctx.files {
                let content = content.replace('\\', "\\\\").replace('"', "\\\"");
                // Truncate very long files in setup
                let content = if content.len() > 5000 {
                    format!("{}...[truncated, use search() for full access]", &content[..5000])
                } else {
                    content
                };
                code.push_str(&format!("    \"{}\": \"\"\"{}\"\"\",\n", path, content));
            }
            code.push_str("}\n\n");
        }

        // Set up tool outputs
        if self.variables.contains_key("tool_outputs") {
            code.push_str("tool_outputs = [\n");
            for output in &ctx.tool_outputs {
                let content = output.content.replace('\\', "\\\\").replace('"', "\\\"");
                let content = if content.len() > 2000 {
                    format!("{}...[truncated]", &content[..2000])
                } else {
                    content
                };
                code.push_str(&format!(
                    "    {{\"tool\": \"{}\", \"content\": \"{}\", \"exit_code\": {}}},\n",
                    output.tool_name,
                    content,
                    output.exit_code.unwrap_or(0)
                ));
            }
            code.push_str("]\n\n");
        }

        // Set up working memory
        if self.variables.contains_key("working_memory") {
            code.push_str("working_memory = ");
            code.push_str(&serde_json::to_string_pretty(&ctx.working_memory).unwrap_or_else(|_| "{}".to_string()));
            code.push_str("\n\n");
        }

        code
    }

    /// Check if total context size is within limits.
    pub fn is_within_limits(&self) -> bool {
        self.total_size_bytes <= REQUIRE_CHUNKING_BYTES
    }

    /// Get variables that require chunking.
    pub fn variables_requiring_chunking(&self) -> Vec<&ContextVariable> {
        self.variables
            .values()
            .filter(|v| v.requires_chunking)
            .collect()
    }
}

/// Configuration for context externalization.
#[derive(Debug, Clone)]
pub struct ExternalizationConfig {
    /// Whether to externalize conversation history.
    pub externalize_conversation: bool,
    /// Whether to externalize cached files.
    pub externalize_files: bool,
    /// Whether to externalize tool outputs.
    pub externalize_tool_outputs: bool,
    /// Whether to externalize working memory.
    pub externalize_working_memory: bool,
}

impl Default for ExternalizationConfig {
    fn default() -> Self {
        Self {
            externalize_conversation: true,
            externalize_files: true,
            externalize_tool_outputs: true,
            externalize_working_memory: true,
        }
    }
}

/// Configuration for root prompt generation.
#[derive(Debug, Clone)]
pub struct RootPromptConfig {
    /// Include documentation for helper functions.
    pub include_helper_docs: bool,
    /// Include warnings about large context.
    pub include_warnings: bool,
    /// Include instructions about not pasting full context.
    pub include_instructions: bool,
}

impl Default for RootPromptConfig {
    fn default() -> Self {
        Self {
            include_helper_docs: true,
            include_warnings: true,
            include_instructions: true,
        }
    }
}

/// Variable access helper definitions for REPL (SPEC-25.03).
///
/// These are Python functions that should be available in the REPL.
#[derive(Debug, Clone)]
pub struct VariableAccessHelper {
    /// Function name.
    pub name: &'static str,
    /// Function signature.
    pub signature: &'static str,
    /// Brief description.
    pub description: &'static str,
    /// Python implementation.
    pub implementation: &'static str,
}

impl VariableAccessHelper {
    /// Get all standard helpers.
    pub fn standard_helpers() -> Vec<Self> {
        vec![
            Self {
                name: "peek",
                signature: "peek(messages, start=0, end=None)",
                description: "Slice messages by index range",
                implementation: r#"
def peek(messages, start=0, end=None):
    """Slice messages from a conversation.

    Args:
        messages: List of message dicts with 'role' and 'content'
        start: Start index (default: 0)
        end: End index (default: None = all remaining)

    Returns:
        List of messages in the range
    """
    if end is None:
        return messages[start:]
    return messages[start:end]
"#,
            },
            Self {
                name: "search",
                signature: "search(files, pattern)",
                description: "Search files by regex pattern",
                implementation: r#"
def search(files, pattern):
    """Search for pattern in files.

    Args:
        files: Dict mapping path -> content
        pattern: Regex pattern to search for

    Returns:
        Dict of path -> list of matching lines
    """
    import re
    results = {}
    regex = re.compile(pattern)
    for path, content in files.items():
        matches = []
        for i, line in enumerate(content.split('\n'), 1):
            if regex.search(line):
                matches.append((i, line))
        if matches:
            results[path] = matches
    return results
"#,
            },
            Self {
                name: "summarize",
                signature: "summarize(item, max_len=500)",
                description: "Summarize a tool output or message",
                implementation: r#"
def summarize(item, max_len=500):
    """Summarize an item (message or tool output).

    Args:
        item: Dict with 'content' key
        max_len: Maximum length of summary

    Returns:
        Truncated content with indicator if truncated
    """
    content = item.get('content', str(item))
    if len(content) <= max_len:
        return content
    return content[:max_len] + f'... [{len(content) - max_len} more chars]'
"#,
            },
            Self {
                name: "grep",
                signature: "grep(files, pattern, context=2)",
                description: "Grep files with context lines",
                implementation: r#"
def grep(files, pattern, context=2):
    """Grep for pattern in files with context.

    Args:
        files: Dict mapping path -> content
        pattern: Regex pattern to search for
        context: Number of context lines before/after

    Returns:
        Dict of path -> list of (line_num, line, context_before, context_after)
    """
    import re
    results = {}
    regex = re.compile(pattern)
    for path, content in files.items():
        lines = content.split('\n')
        matches = []
        for i, line in enumerate(lines):
            if regex.search(line):
                before = lines[max(0, i-context):i]
                after = lines[i+1:min(len(lines), i+1+context)]
                matches.append({
                    'line_num': i + 1,
                    'line': line,
                    'before': before,
                    'after': after
                })
        if matches:
            results[path] = matches
    return results
"#,
            },
            Self {
                name: "file_tree",
                signature: "file_tree(files)",
                description: "Show file tree structure",
                implementation: r#"
def file_tree(files):
    """Show tree structure of files.

    Args:
        files: Dict mapping path -> content

    Returns:
        String representation of file tree
    """
    from collections import defaultdict
    import os

    tree = defaultdict(list)
    for path in sorted(files.keys()):
        parts = path.split(os.sep)
        for i in range(len(parts)):
            parent = os.sep.join(parts[:i]) or '.'
            child = parts[i]
            if child not in tree[parent]:
                tree[parent].append(child)

    def render(path, prefix=''):
        result = []
        children = tree.get(path, [])
        for i, child in enumerate(children):
            is_last = i == len(children) - 1
            connector = '└── ' if is_last else '├── '
            result.append(f'{prefix}{connector}{child}')
            child_path = f'{path}{os.sep}{child}' if path != '.' else child
            if child_path in tree:
                extension = '    ' if is_last else '│   '
                result.extend(render(child_path, prefix + extension))
        return result

    return '\n'.join(render('.'))
"#,
            },
        ]
    }

    /// Generate Python module code for all helpers.
    pub fn generate_module() -> String {
        let mut code = String::new();
        code.push_str("\"\"\"RLM context access helpers.\n\n");
        code.push_str("These functions help efficiently access externalized context.\n");
        code.push_str("\"\"\"\n\n");

        for helper in Self::standard_helpers() {
            code.push_str(helper.implementation);
            code.push('\n');
        }

        code.push_str("\n__all__ = ['peek', 'search', 'summarize', 'grep', 'file_tree']\n");
        code
    }
}

/// Context size tracker for monitoring and limits (SPEC-25.04).
#[derive(Debug, Clone, Default)]
pub struct ContextSizeTracker {
    /// Size history by variable name.
    pub history: HashMap<String, Vec<usize>>,
    /// Current sizes.
    pub current: HashMap<String, usize>,
    /// Total bytes tracked.
    pub total_bytes: usize,
}

impl ContextSizeTracker {
    /// Create a new tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update size for a variable.
    pub fn update(&mut self, name: &str, size: usize) {
        // Update history
        self.history
            .entry(name.to_string())
            .or_default()
            .push(size);

        // Update current
        if let Some(old_size) = self.current.insert(name.to_string(), size) {
            self.total_bytes = self.total_bytes.saturating_sub(old_size);
        }
        self.total_bytes += size;
    }

    /// Check if a variable exceeds the warning threshold.
    pub fn exceeds_warning(&self, name: &str) -> bool {
        self.current.get(name).copied().unwrap_or(0) > WARN_SIZE_BYTES
    }

    /// Check if a variable requires chunking.
    pub fn requires_chunking(&self, name: &str) -> bool {
        self.current.get(name).copied().unwrap_or(0) > REQUIRE_CHUNKING_BYTES
    }

    /// Get growth rate for a variable (bytes per update).
    pub fn growth_rate(&self, name: &str) -> Option<f64> {
        let history = self.history.get(name)?;
        if history.len() < 2 {
            return None;
        }
        let growth: f64 = history
            .windows(2)
            .map(|w| w[1] as f64 - w[0] as f64)
            .sum();
        Some(growth / (history.len() - 1) as f64)
    }

    /// Generate warnings for all variables.
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        for (name, &size) in &self.current {
            if size > REQUIRE_CHUNKING_BYTES {
                warnings.push(format!(
                    "{} exceeds {}MB ({:.1}MB) - chunking required",
                    name,
                    REQUIRE_CHUNKING_BYTES / (1024 * 1024),
                    size as f64 / (1024.0 * 1024.0)
                ));
            } else if size > WARN_SIZE_BYTES {
                warnings.push(format!(
                    "{} exceeds {}KB ({:.1}KB)",
                    name,
                    WARN_SIZE_BYTES / 1024,
                    size as f64 / 1024.0
                ));
            }
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_variable_creation() {
        let var = ContextVariable::new("conversation", ContextVarType::Conversation, 50000, 100);
        assert_eq!(var.name, "conversation");
        assert!(!var.size_warning); // 50KB < 100KB
        assert!(!var.requires_chunking);
        assert!(var.summary.contains("100 messages"));
    }

    #[test]
    fn test_context_variable_size_warning() {
        let var = ContextVariable::new("files", ContextVarType::Files, 150 * 1024, 10);
        assert!(var.size_warning); // 150KB > 100KB
        assert!(!var.requires_chunking); // 150KB < 1MB
    }

    #[test]
    fn test_context_variable_requires_chunking() {
        let var = ContextVariable::new("files", ContextVarType::Files, 2 * 1024 * 1024, 50);
        assert!(var.size_warning);
        assert!(var.requires_chunking); // 2MB > 1MB
    }

    #[test]
    fn test_externalized_context_from_session() {
        let mut ctx = SessionContext::new();
        ctx.add_user_message("Hello");
        ctx.add_assistant_message("Hi there!");
        ctx.cache_file("/src/main.rs", "fn main() {}");

        let externalized = ExternalizedContext::from_session(&ctx, "Test query");

        assert_eq!(externalized.query, "Test query");
        assert!(externalized.variables.contains_key("conversation"));
        assert!(externalized.variables.contains_key("files"));
        assert_eq!(externalized.variables["conversation"].item_count, 2);
        assert_eq!(externalized.variables["files"].item_count, 1);
    }

    #[test]
    fn test_root_prompt_generation() {
        let mut ctx = SessionContext::new();
        ctx.add_user_message("Analyze auth");
        ctx.cache_file("/src/auth.rs", "fn auth() {}");

        let externalized = ExternalizedContext::from_session(&ctx, "Analyze the auth system");
        let prompt = externalized.root_prompt();

        assert!(prompt.contains("## Query"));
        assert!(prompt.contains("Analyze the auth system"));
        assert!(prompt.contains("## Available Context Variables"));
        assert!(prompt.contains("conversation"));
        assert!(prompt.contains("files"));
        assert!(prompt.contains("peek("));
        assert!(prompt.contains("search("));
    }

    #[test]
    fn test_externalization_config() {
        let mut ctx = SessionContext::new();
        ctx.add_user_message("Test");
        ctx.cache_file("/test.rs", "test");

        let config = ExternalizationConfig {
            externalize_conversation: false,
            externalize_files: true,
            externalize_tool_outputs: true,
            externalize_working_memory: true,
        };

        let externalized =
            ExternalizedContext::from_session_with_config(&ctx, "Query", &config);

        assert!(!externalized.variables.contains_key("conversation"));
        assert!(externalized.variables.contains_key("files"));
    }

    #[test]
    fn test_size_tracker() {
        let mut tracker = ContextSizeTracker::new();

        tracker.update("conversation", 50_000);
        assert!(!tracker.exceeds_warning("conversation"));

        tracker.update("conversation", 150_000);
        assert!(tracker.exceeds_warning("conversation"));
        assert!(!tracker.requires_chunking("conversation"));

        tracker.update("files", 2_000_000);
        assert!(tracker.requires_chunking("files"));

        let warnings = tracker.warnings();
        assert!(warnings.iter().any(|w| w.contains("conversation")));
        assert!(warnings.iter().any(|w| w.contains("files")));
    }

    #[test]
    fn test_growth_rate() {
        let mut tracker = ContextSizeTracker::new();

        tracker.update("conversation", 1000);
        assert!(tracker.growth_rate("conversation").is_none()); // Need 2+ points

        tracker.update("conversation", 2000);
        tracker.update("conversation", 3000);
        tracker.update("conversation", 4000);

        let rate = tracker.growth_rate("conversation").unwrap();
        assert!((rate - 1000.0).abs() < 0.01); // 1000 bytes per update
    }

    #[test]
    fn test_helper_module_generation() {
        let module = VariableAccessHelper::generate_module();

        assert!(module.contains("def peek("));
        assert!(module.contains("def search("));
        assert!(module.contains("def summarize("));
        assert!(module.contains("def grep("));
        assert!(module.contains("def file_tree("));
        assert!(module.contains("__all__"));
    }

    #[test]
    fn test_repl_setup_code() {
        let mut ctx = SessionContext::new();
        ctx.add_user_message("Test message");
        ctx.cache_file("/src/test.rs", "fn test() {}");

        let externalized = ExternalizedContext::from_session(&ctx, "Query");
        let setup = externalized.repl_setup_code(&ctx);

        assert!(setup.contains("conversation = ["));
        assert!(setup.contains("files = {"));
        assert!(setup.contains("from rlm_helpers import"));
    }
}
