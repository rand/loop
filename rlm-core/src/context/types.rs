//! Core context types: Message, ToolOutput, SessionContext.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// The role of a message participant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System instructions
    System,
    /// User/human input
    User,
    /// Assistant/model response
    Assistant,
    /// Tool execution result
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A message in the conversation history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,
    /// Content of the message
    pub content: String,
    /// When the message was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    /// Additional metadata (tool_use_id, citations, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

impl Message {
    /// Create a new message with just role and content.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            timestamp: Some(Utc::now()),
            metadata: None,
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// Create a tool result message.
    pub fn tool(content: impl Into<String>) -> Self {
        Self::new(Role::Tool, content)
    }

    /// Add metadata to the message.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Get a metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.as_ref()?.get(key)
    }

    /// Approximate token count for the message content.
    /// Uses rough heuristic: ~4 chars per token for English text.
    pub fn approx_tokens(&self) -> usize {
        self.content.len() / 4
    }
}

/// Output from a tool execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Name of the tool that was executed
    pub tool_name: String,
    /// Output content from the tool
    pub content: String,
    /// Exit code if applicable (e.g., for bash commands)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// When the tool was executed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
    /// Tool-specific metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

impl ToolOutput {
    /// Create a new tool output.
    pub fn new(tool_name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            content: content.into(),
            exit_code: None,
            timestamp: Some(Utc::now()),
            metadata: None,
        }
    }

    /// Set the exit code.
    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.exit_code = Some(code);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Check if the tool execution succeeded (exit_code == 0 or None).
    pub fn is_success(&self) -> bool {
        self.exit_code.map_or(true, |code| code == 0)
    }

    /// Approximate token count for the output content.
    pub fn approx_tokens(&self) -> usize {
        self.content.len() / 4
    }
}

/// The full session context for RLM orchestration.
///
/// Contains all the externalized variables that the RLM loop operates on:
/// - `messages`: Full conversation history
/// - `files`: Cached file contents by path
/// - `tool_outputs`: Recent tool execution results
/// - `working_memory`: Session-scoped key-value state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionContext {
    /// Conversation messages
    pub messages: Vec<Message>,
    /// Cached file contents (path -> content)
    pub files: HashMap<String, String>,
    /// Recent tool outputs
    pub tool_outputs: Vec<ToolOutput>,
    /// Working memory (session state)
    pub working_memory: HashMap<String, Value>,
}

impl SessionContext {
    /// Create a new empty session context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Add a user message.
    pub fn add_user_message(&mut self, content: impl Into<String>) {
        self.messages.push(Message::user(content));
    }

    /// Add an assistant message.
    pub fn add_assistant_message(&mut self, content: impl Into<String>) {
        self.messages.push(Message::assistant(content));
    }

    /// Cache a file's contents.
    pub fn cache_file(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    /// Get cached file content.
    pub fn get_file(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }

    /// Add a tool output.
    pub fn add_tool_output(&mut self, output: ToolOutput) {
        self.tool_outputs.push(output);
    }

    /// Set a working memory value.
    pub fn set_memory(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        self.working_memory.insert(key.into(), value.into());
    }

    /// Get a working memory value.
    pub fn get_memory(&self, key: &str) -> Option<&Value> {
        self.working_memory.get(key)
    }

    /// Get the last N messages.
    pub fn last_messages(&self, n: usize) -> &[Message] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }

    /// Get the last user message.
    pub fn last_user_message(&self) -> Option<&Message> {
        self.messages.iter().rev().find(|m| m.role == Role::User)
    }

    /// Get the last assistant message.
    pub fn last_assistant_message(&self) -> Option<&Message> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
    }

    /// Count total approximate tokens in messages.
    pub fn total_message_tokens(&self) -> usize {
        self.messages.iter().map(|m| m.approx_tokens()).sum()
    }

    /// Count total approximate tokens in cached files.
    pub fn total_file_tokens(&self) -> usize {
        self.files.values().map(|c| c.len() / 4).sum()
    }

    /// Count total approximate tokens in tool outputs.
    pub fn total_tool_tokens(&self) -> usize {
        self.tool_outputs.iter().map(|o| o.approx_tokens()).sum()
    }

    /// Get unique file paths referenced.
    pub fn file_paths(&self) -> Vec<&str> {
        self.files.keys().map(|s| s.as_str()).collect()
    }

    /// Check if context spans multiple directories.
    pub fn spans_multiple_directories(&self) -> bool {
        let dirs: std::collections::HashSet<_> = self
            .files
            .keys()
            .filter_map(|path| {
                std::path::Path::new(path)
                    .parent()
                    .and_then(|p| p.to_str())
            })
            .collect();
        dirs.len() > 1
    }

    /// Clear tool outputs (keeping last N).
    pub fn trim_tool_outputs(&mut self, keep_last: usize) {
        if self.tool_outputs.len() > keep_last {
            let start = self.tool_outputs.len() - keep_last;
            self.tool_outputs = self.tool_outputs.split_off(start);
        }
    }

    /// Clear working memory.
    pub fn clear_working_memory(&mut self) {
        self.working_memory.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello, world!");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello, world!");
        assert!(msg.timestamp.is_some());
    }

    #[test]
    fn test_message_with_metadata() {
        let msg = Message::assistant("Response")
            .with_metadata("model", "claude-3-opus")
            .with_metadata("tokens", 150);

        assert_eq!(
            msg.get_metadata("model"),
            Some(&Value::String("claude-3-opus".into()))
        );
        assert_eq!(msg.get_metadata("tokens"), Some(&Value::Number(150.into())));
    }

    #[test]
    fn test_tool_output() {
        let output = ToolOutput::new("bash", "Hello\n").with_exit_code(0);

        assert!(output.is_success());
        assert_eq!(output.tool_name, "bash");
    }

    #[test]
    fn test_session_context() {
        let mut ctx = SessionContext::new();

        ctx.add_user_message("Hello");
        ctx.add_assistant_message("Hi there!");
        ctx.cache_file("/src/main.rs", "fn main() {}");
        ctx.set_memory("depth", 0);

        assert_eq!(ctx.messages.len(), 2);
        assert_eq!(ctx.get_file("/src/main.rs"), Some("fn main() {}"));
        assert_eq!(ctx.get_memory("depth"), Some(&Value::Number(0.into())));
    }

    #[test]
    fn test_spans_multiple_directories() {
        let mut ctx = SessionContext::new();
        ctx.cache_file("/src/lib.rs", "");
        assert!(!ctx.spans_multiple_directories());

        ctx.cache_file("/tests/test.rs", "");
        assert!(ctx.spans_multiple_directories());
    }

    #[test]
    fn test_last_messages() {
        let mut ctx = SessionContext::new();
        ctx.add_user_message("First");
        ctx.add_assistant_message("Second");
        ctx.add_user_message("Third");

        let last_two = ctx.last_messages(2);
        assert_eq!(last_two.len(), 2);
        assert_eq!(last_two[0].content, "Second");
        assert_eq!(last_two[1].content, "Third");
    }
}
