//! Topos MCP client for connecting to the Topos MCP server.
//!
//! This module provides a client for interacting with the Topos MCP server,
//! enabling spec validation, context compilation, and semantic analysis.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::error::{Error, Result};

/// Configuration for the Topos MCP client.
#[derive(Debug, Clone)]
pub struct ToposClientConfig {
    /// Path to the topos-mcp binary.
    pub binary_path: Option<String>,
    /// Server URL for HTTP transport (alternative to stdio).
    pub server_url: Option<String>,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Whether to auto-start the server if not running.
    pub auto_start: bool,
}

impl Default for ToposClientConfig {
    fn default() -> Self {
        Self {
            binary_path: None,
            server_url: None,
            timeout_ms: 30_000,
            auto_start: true,
        }
    }
}

impl ToposClientConfig {
    /// Create configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            binary_path: std::env::var("TOPOS_MCP_BINARY").ok(),
            server_url: std::env::var("TOPOS_MCP_URL").ok(),
            timeout_ms: std::env::var("TOPOS_MCP_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30_000),
            auto_start: std::env::var("TOPOS_MCP_AUTO_START")
                .map(|s| s != "0" && s.to_lowercase() != "false")
                .unwrap_or(true),
        }
    }
}

/// Result from validate_spec tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the spec is valid (no errors).
    pub valid: bool,
    /// List of diagnostics.
    pub diagnostics: Vec<Diagnostic>,
    /// Raw output text.
    pub raw_output: String,
}

/// A diagnostic message from validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: DiagnosticSeverity,
    /// Line number (1-indexed).
    pub line: u32,
    /// Diagnostic message.
    pub message: String,
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// Result from summarize_spec tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecSummary {
    /// Spec file path.
    pub path: String,
    /// Number of requirements.
    pub requirement_count: usize,
    /// Number of concepts.
    pub concept_count: usize,
    /// Number of behaviors.
    pub behavior_count: usize,
    /// Number of tasks.
    pub task_count: usize,
    /// Requirements without tasks.
    pub untasked_requirements: Vec<String>,
    /// Raw markdown summary.
    pub raw_output: String,
}

/// Result from compile_context tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledContext {
    /// Task ID that was compiled.
    pub task_id: String,
    /// Compiled context content.
    pub content: String,
    /// Output format used.
    pub format: String,
}

/// MCP JSON-RPC request.
#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: Value,
}

/// MCP JSON-RPC response.
#[derive(Debug, Deserialize)]
struct McpResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    id: u64,
    result: Option<Value>,
    error: Option<McpError>,
}

/// MCP error object.
#[derive(Debug, Deserialize)]
struct McpError {
    code: i32,
    message: String,
}

/// Topos MCP client.
pub struct ToposClient {
    config: ToposClientConfig,
    process: Arc<Mutex<Option<McpProcess>>>,
    request_id: Arc<Mutex<u64>>,
}

/// MCP server process handle.
struct McpProcess {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl ToposClient {
    /// Create a new client with the given configuration.
    pub fn new(config: ToposClientConfig) -> Self {
        Self {
            config,
            process: Arc::new(Mutex::new(None)),
            request_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a client from environment variables.
    pub fn from_env() -> Self {
        Self::new(ToposClientConfig::from_env())
    }

    /// Check if the client is connected.
    pub async fn is_connected(&self) -> bool {
        let process = self.process.lock().await;
        process.is_some()
    }

    /// Connect to the MCP server.
    pub async fn connect(&self) -> Result<()> {
        let mut process_guard = self.process.lock().await;

        if process_guard.is_some() {
            return Ok(());
        }

        let binary = self.find_binary()?;

        let mut child = Command::new(&binary)
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Internal(format!("Failed to start topos-mcp: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Internal("Failed to get stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Internal("Failed to get stdout".to_string()))?;

        *process_guard = Some(McpProcess {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        });

        // Initialize the connection
        drop(process_guard);
        self.initialize().await?;

        Ok(())
    }

    /// Disconnect from the MCP server.
    pub async fn disconnect(&self) -> Result<()> {
        let mut process_guard = self.process.lock().await;

        if let Some(mut process) = process_guard.take() {
            let _ = process.child.kill().await;
        }

        Ok(())
    }

    /// Find the topos-mcp binary.
    fn find_binary(&self) -> Result<String> {
        // Check config first
        if let Some(ref path) = self.config.binary_path {
            return Ok(path.clone());
        }

        // Check PATH
        if let Ok(path) = which::which("topos") {
            return Ok(path.to_string_lossy().to_string());
        }

        // Check common locations
        let common_paths = [
            "/usr/local/bin/topos",
            "/opt/homebrew/bin/topos",
            "~/.cargo/bin/topos",
        ];

        for path in common_paths {
            let expanded = shellexpand::tilde(path);
            if std::path::Path::new(expanded.as_ref()).exists() {
                return Ok(expanded.to_string());
            }
        }

        Err(Error::Config(
            "topos-mcp binary not found. Set TOPOS_MCP_BINARY or install topos.".to_string(),
        ))
    }

    /// Initialize the MCP connection.
    async fn initialize(&self) -> Result<()> {
        let _response = self
            .call_method(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "rlm-core",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )
            .await?;

        // Send initialized notification
        self.send_notification("notifications/initialized", json!({}))
            .await?;

        Ok(())
    }

    /// Get the next request ID.
    async fn next_id(&self) -> u64 {
        let mut id = self.request_id.lock().await;
        *id += 1;
        *id
    }

    /// Call an MCP method.
    async fn call_method(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id().await;

        let request = McpRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let request_json =
            serde_json::to_string(&request).map_err(|e| Error::Serialization(e))?;

        let mut process_guard = self.process.lock().await;
        let process = process_guard
            .as_mut()
            .ok_or_else(|| Error::Internal("Not connected".to_string()))?;

        // Send request
        process
            .stdin
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| Error::SubprocessComm(format!("Write error: {}", e)))?;
        process
            .stdin
            .write_all(b"\n")
            .await
            .map_err(|e| Error::SubprocessComm(format!("Write error: {}", e)))?;
        process
            .stdin
            .flush()
            .await
            .map_err(|e| Error::SubprocessComm(format!("Flush error: {}", e)))?;

        // Read response
        let mut response_line = String::new();
        process
            .stdout
            .read_line(&mut response_line)
            .await
            .map_err(|e| Error::SubprocessComm(format!("Read error: {}", e)))?;

        let response: McpResponse =
            serde_json::from_str(&response_line).map_err(|e| Error::Serialization(e))?;

        if response.id != id {
            return Err(Error::Internal(format!(
                "Response ID mismatch: expected {}, got {}",
                id, response.id
            )));
        }

        if let Some(error) = response.error {
            return Err(Error::Internal(format!(
                "MCP error {}: {}",
                error.code, error.message
            )));
        }

        response
            .result
            .ok_or_else(|| Error::Internal("Empty response".to_string()))
    }

    /// Send an MCP notification.
    async fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let notification_json =
            serde_json::to_string(&notification).map_err(|e| Error::Serialization(e))?;

        let mut process_guard = self.process.lock().await;
        let process = process_guard
            .as_mut()
            .ok_or_else(|| Error::Internal("Not connected".to_string()))?;

        process
            .stdin
            .write_all(notification_json.as_bytes())
            .await
            .map_err(|e| Error::SubprocessComm(format!("Write error: {}", e)))?;
        process
            .stdin
            .write_all(b"\n")
            .await
            .map_err(|e| Error::SubprocessComm(format!("Write error: {}", e)))?;
        process
            .stdin
            .flush()
            .await
            .map_err(|e| Error::SubprocessComm(format!("Flush error: {}", e)))?;

        Ok(())
    }

    /// Call a tool on the MCP server.
    async fn call_tool(&self, name: &str, arguments: HashMap<String, Value>) -> Result<String> {
        let result = self
            .call_method(
                "tools/call",
                json!({
                    "name": name,
                    "arguments": arguments
                }),
            )
            .await?;

        // Extract text content from the response
        let content = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        Ok(content.to_string())
    }

    // =========================================================================
    // Public API - Tool Methods
    // =========================================================================

    /// Validate a Topos specification file.
    pub async fn validate_spec(&self, path: &Path) -> Result<ValidationResult> {
        if !self.is_connected().await {
            self.connect().await?;
        }

        let mut args = HashMap::new();
        args.insert("path".to_string(), json!(path.to_string_lossy()));

        let output = self.call_tool("validate_spec", args).await?;

        // Parse the output
        let valid = output.contains("No errors found");
        let diagnostics = Self::parse_diagnostics(&output);

        Ok(ValidationResult {
            valid,
            diagnostics,
            raw_output: output,
        })
    }

    /// Get a summary of a Topos specification.
    pub async fn summarize_spec(&self, path: &Path) -> Result<SpecSummary> {
        if !self.is_connected().await {
            self.connect().await?;
        }

        let mut args = HashMap::new();
        args.insert("path".to_string(), json!(path.to_string_lossy()));

        let output = self.call_tool("summarize_spec", args).await?;

        // Parse counts from the output
        let (req_count, concept_count, behavior_count, task_count) = Self::parse_counts(&output);
        let untasked = Self::parse_untasked(&output);

        Ok(SpecSummary {
            path: path.to_string_lossy().to_string(),
            requirement_count: req_count,
            concept_count,
            behavior_count,
            task_count,
            untasked_requirements: untasked,
            raw_output: output,
        })
    }

    /// Compile context for a specific task.
    pub async fn compile_context(
        &self,
        path: &Path,
        task_id: &str,
        format: Option<&str>,
    ) -> Result<CompiledContext> {
        if !self.is_connected().await {
            self.connect().await?;
        }

        let mut args = HashMap::new();
        args.insert("path".to_string(), json!(path.to_string_lossy()));
        args.insert("task_id".to_string(), json!(task_id));
        if let Some(fmt) = format {
            args.insert("format".to_string(), json!(fmt));
        }

        let output = self.call_tool("compile_context", args).await?;

        Ok(CompiledContext {
            task_id: task_id.to_string(),
            content: output,
            format: format.unwrap_or("markdown").to_string(),
        })
    }

    /// Get suggestions for a typed hole.
    pub async fn suggest_hole(
        &self,
        path: &Path,
        line: Option<u32>,
        column: Option<u32>,
    ) -> Result<String> {
        if !self.is_connected().await {
            self.connect().await?;
        }

        let mut args = HashMap::new();
        args.insert("path".to_string(), json!(path.to_string_lossy()));
        if let Some(l) = line {
            args.insert("line".to_string(), json!(l));
        }
        if let Some(c) = column {
            args.insert("column".to_string(), json!(c));
        }

        self.call_tool("suggest_hole", args).await
    }

    /// Extract spec from Rust files.
    pub async fn extract_spec(
        &self,
        paths: Vec<&Path>,
        spec_name: Option<&str>,
    ) -> Result<String> {
        if !self.is_connected().await {
            self.connect().await?;
        }

        let mut args = HashMap::new();
        let path_strs: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
        args.insert("paths".to_string(), json!(path_strs));
        if let Some(name) = spec_name {
            args.insert("spec_name".to_string(), json!(name));
        }

        self.call_tool("extract_spec", args).await
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Parse diagnostics from validation output.
    fn parse_diagnostics(output: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for line in output.lines() {
            // Format: "- [SEVERITY] Line N: message"
            if line.starts_with("- [") {
                let severity = if line.contains("[ERROR]") {
                    DiagnosticSeverity::Error
                } else if line.contains("[WARNING]") {
                    DiagnosticSeverity::Warning
                } else {
                    DiagnosticSeverity::Info
                };

                // Extract line number
                if let Some(line_start) = line.find("Line ") {
                    let rest = &line[line_start + 5..];
                    if let Some(colon_pos) = rest.find(':') {
                        if let Ok(line_num) = rest[..colon_pos].trim().parse::<u32>() {
                            let message = rest[colon_pos + 1..].trim().to_string();
                            diagnostics.push(Diagnostic {
                                severity,
                                line: line_num,
                                message,
                            });
                        }
                    }
                }
            }
        }

        diagnostics
    }

    /// Parse counts from summary output.
    fn parse_counts(output: &str) -> (usize, usize, usize, usize) {
        let mut req = 0;
        let mut concepts = 0;
        let mut behaviors = 0;
        let mut tasks = 0;

        for line in output.lines() {
            if line.contains("Requirements") && line.contains("total") {
                if let Some(num) = Self::extract_number(line) {
                    req = num;
                }
            } else if line.contains("Concepts") {
                if let Some(num) = Self::extract_number(line) {
                    concepts = num;
                }
            } else if line.contains("Behaviors") {
                if let Some(num) = Self::extract_number(line) {
                    behaviors = num;
                }
            } else if line.contains("Tasks") {
                if let Some(num) = Self::extract_number(line) {
                    tasks = num;
                }
            }
        }

        (req, concepts, behaviors, tasks)
    }

    /// Extract a number from a line.
    fn extract_number(line: &str) -> Option<usize> {
        line.split_whitespace()
            .filter_map(|word| word.parse::<usize>().ok())
            .next()
    }

    /// Parse untasked requirements from summary output.
    fn parse_untasked(output: &str) -> Vec<String> {
        let mut untasked = Vec::new();

        for line in output.lines() {
            if line.contains("Without tasks") {
                // Format: "- **Without tasks**: N (REQ-1, REQ-2)"
                if let Some(paren_start) = line.find('(') {
                    if let Some(paren_end) = line.find(')') {
                        let reqs = &line[paren_start + 1..paren_end];
                        if reqs != "all covered" {
                            for req in reqs.split(',') {
                                let trimmed = req.trim();
                                if !trimmed.is_empty() {
                                    untasked.push(trimmed.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        untasked
    }
}

impl Drop for ToposClient {
    fn drop(&mut self) {
        // Best-effort cleanup - we can't do async in drop
        // The process will be cleaned up by the OS anyway
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ToposClientConfig::default();
        assert!(config.binary_path.is_none());
        assert!(config.auto_start);
        assert_eq!(config.timeout_ms, 30_000);
    }

    #[test]
    fn test_parse_diagnostics() {
        let output = r#"Found 2 issue(s) in test.tps:

- [ERROR] Line 5: Undefined reference `Foo`
- [WARNING] Line 10: Unused concept `Bar`
"#;

        let diags = ToposClient::parse_diagnostics(output);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
        assert_eq!(diags[0].line, 5);
        assert_eq!(diags[1].severity, DiagnosticSeverity::Warning);
        assert_eq!(diags[1].line, 10);
    }

    #[test]
    fn test_parse_counts() {
        let output = r#"## Traceability

- **Requirements**: 5 total
- **Without tasks**: 2 (REQ-1, REQ-3)
- **Tasks**: 3
- **Concepts**: 4
- **Behaviors**: 6
"#;

        let (req, concepts, behaviors, tasks) = ToposClient::parse_counts(output);
        assert_eq!(req, 5);
        assert_eq!(concepts, 4);
        assert_eq!(behaviors, 6);
        assert_eq!(tasks, 3);
    }

    #[test]
    fn test_parse_untasked() {
        let output = "- **Without tasks**: 2 (REQ-1, REQ-3)";
        let untasked = ToposClient::parse_untasked(output);
        assert_eq!(untasked, vec!["REQ-1", "REQ-3"]);
    }
}
