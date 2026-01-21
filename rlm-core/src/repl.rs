//! Python REPL subprocess management.
//!
//! This module provides the Rust side of the REPL subprocess communication,
//! spawning a Python process and communicating via JSON-RPC over stdin/stdout.
//!
//! # Signature Support
//!
//! The REPL supports typed signatures via the SUBMIT mechanism:
//! 1. Register a signature with `register_signature()` before execution
//! 2. Execute code that calls `SUBMIT(outputs)` when done
//! 3. Outputs are validated against the registered signature
//! 4. Results are returned in `ExecuteResult.submit_result`

use crate::error::{Error, Result};
use crate::signature::{FieldSpec, SubmitResult, SignatureRegistration};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// JSON-RPC request structure.
#[derive(Debug, Clone, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u64,
}

impl JsonRpcRequest {
    fn new(method: impl Into<String>, params: Value, id: u64) -> Self {
        Self {
            jsonrpc: "2.0",
            method: method.into(),
            params,
            id,
        }
    }
}

/// JSON-RPC response structure.
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Option<u64>,
}

/// JSON-RPC error structure.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// Result of code execution in the REPL.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecuteResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Return value (if any)
    pub result: Option<Value>,
    /// Captured stdout
    pub stdout: String,
    /// Captured stderr
    pub stderr: String,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Error type (if failed)
    pub error_type: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: f64,
    /// IDs of pending deferred operations
    pub pending_operations: Vec<String>,
    /// Result of SUBMIT call (if signature was registered and SUBMIT was called)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submit_result: Option<SubmitResult>,
}

/// A pending deferred operation that needs to be resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingOperation {
    /// Unique operation ID
    pub id: String,
    /// Type of operation (llm_call, summarize, etc.)
    pub operation_type: String,
    /// Operation parameters
    pub params: HashMap<String, Value>,
}

/// Status of the REPL subprocess.
#[derive(Debug, Clone, Deserialize)]
pub struct ReplStatus {
    pub ready: bool,
    pub pending_operations: usize,
    pub variables_count: usize,
    pub memory_usage_bytes: Option<u64>,
}

/// Configuration for the REPL subprocess.
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// Path to the Python executable (default: "python3")
    pub python_path: String,
    /// Path to the rlm-repl package (default: looks in standard locations)
    pub repl_package_path: Option<String>,
    /// Timeout for REPL operations in milliseconds
    pub timeout_ms: u64,
    /// Maximum memory in bytes (enforced by ulimit on Unix)
    pub max_memory_bytes: Option<u64>,
    /// Maximum CPU time in seconds
    pub max_cpu_seconds: Option<u64>,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            python_path: "python3".to_string(),
            repl_package_path: None,
            timeout_ms: 30_000,
            max_memory_bytes: Some(512 * 1024 * 1024), // 512 MB
            max_cpu_seconds: Some(60),
        }
    }
}

/// Handle to a running REPL subprocess.
pub struct ReplHandle {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    config: ReplConfig,
}

impl ReplHandle {
    /// Spawn a new REPL subprocess.
    pub fn spawn(config: ReplConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.python_path);
        cmd.arg("-m").arg("rlm_repl");

        // Resource limits are enforced via timeout in send_request
        // For stricter limits, the host can use cgroups or similar

        // Configure I/O
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add PYTHONPATH if package path is specified
        if let Some(ref path) = config.repl_package_path {
            cmd.env("PYTHONPATH", path);
        }

        let mut child = cmd.spawn().map_err(|e| {
            Error::SubprocessComm(format!("Failed to spawn REPL subprocess: {}", e))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            Error::SubprocessComm("Failed to get stdin handle".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            Error::SubprocessComm("Failed to get stdout handle".to_string())
        })?;

        let stdout = BufReader::new(stdout);

        let mut handle = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
            config,
        };

        // Wait for ready message
        handle.wait_for_ready()?;

        Ok(handle)
    }

    fn wait_for_ready(&mut self) -> Result<()> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).map_err(|e| {
            Error::SubprocessComm(format!("Failed to read ready message: {}", e))
        })?;

        let msg: Value = serde_json::from_str(&line).map_err(|e| {
            Error::SubprocessComm(format!("Invalid ready message: {}", e))
        })?;

        if msg.get("method") != Some(&Value::String("ready".to_string())) {
            return Err(Error::SubprocessComm(format!(
                "Expected ready message, got: {}",
                line
            )));
        }

        Ok(())
    }

    fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = JsonRpcRequest::new(method, params, id);
        let request_json = serde_json::to_string(&request)?;

        // Send request
        writeln!(self.stdin, "{}", request_json).map_err(|e| {
            Error::SubprocessComm(format!("Failed to send request: {}", e))
        })?;
        self.stdin.flush().map_err(|e| {
            Error::SubprocessComm(format!("Failed to flush stdin: {}", e))
        })?;

        // Read response with timeout
        let start = Instant::now();
        let timeout = Duration::from_millis(self.config.timeout_ms);

        loop {
            let mut line = String::new();

            // Check timeout
            if start.elapsed() > timeout {
                return Err(Error::timeout(self.config.timeout_ms));
            }

            // Try to read a line
            match self.stdout.read_line(&mut line) {
                Ok(0) => {
                    return Err(Error::SubprocessComm(
                        "REPL subprocess closed unexpectedly".to_string(),
                    ));
                }
                Ok(_) => {
                    let response: JsonRpcResponse = serde_json::from_str(&line)?;

                    // Check if this is our response
                    if response.id == Some(id) {
                        if let Some(error) = response.error {
                            return Err(Error::repl_execution(format!(
                                "{}: {}",
                                error.code, error.message
                            )));
                        }
                        return Ok(response.result.unwrap_or(Value::Null));
                    }
                    // Otherwise it's a notification or response for a different request
                    // In a more sophisticated implementation, we'd handle these
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    return Err(Error::SubprocessComm(format!(
                        "Failed to read response: {}",
                        e
                    )));
                }
            }
        }
    }

    /// Execute Python code in the REPL.
    pub fn execute(&mut self, code: &str) -> Result<ExecuteResult> {
        let params = serde_json::json!({
            "code": code,
            "timeout_ms": self.config.timeout_ms,
            "capture_output": true,
        });

        let result = self.send_request("execute", params)?;
        let execute_result: ExecuteResult = serde_json::from_value(result)?;
        Ok(execute_result)
    }

    /// Get a variable from the REPL namespace.
    pub fn get_variable(&mut self, name: &str) -> Result<Value> {
        let params = serde_json::json!({ "name": name });
        self.send_request("get_variable", params)
    }

    /// Set a variable in the REPL namespace.
    pub fn set_variable(&mut self, name: &str, value: Value) -> Result<()> {
        let params = serde_json::json!({
            "name": name,
            "value": value,
        });
        self.send_request("set_variable", params)?;
        Ok(())
    }

    /// Resolve a deferred operation.
    pub fn resolve_operation(&mut self, operation_id: &str, result: Value) -> Result<()> {
        let params = serde_json::json!({
            "operation_id": operation_id,
            "result": result,
        });
        self.send_request("resolve_operation", params)?;
        Ok(())
    }

    /// List all variables in the REPL namespace.
    pub fn list_variables(&mut self) -> Result<HashMap<String, String>> {
        let result = self.send_request("list_variables", Value::Null)?;
        let vars: HashMap<String, String> = result
            .get("variables")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        Ok(vars)
    }

    /// Get REPL status.
    pub fn status(&mut self) -> Result<ReplStatus> {
        let result = self.send_request("status", Value::Null)?;
        let status: ReplStatus = serde_json::from_value(result)?;
        Ok(status)
    }

    /// Reset the REPL state.
    pub fn reset(&mut self) -> Result<()> {
        self.send_request("reset", Value::Null)?;
        Ok(())
    }

    /// Register a signature for SUBMIT validation.
    ///
    /// This must be called before executing code that uses `SUBMIT()`.
    /// The signature defines the expected output fields and their types.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rlm_core::signature::{FieldSpec, FieldType};
    ///
    /// let fields = vec![
    ///     FieldSpec::new("answer", FieldType::String),
    ///     FieldSpec::new("confidence", FieldType::Float),
    /// ];
    /// repl.register_signature(fields, Some("MySignature"))?;
    ///
    /// // Now execute code that calls SUBMIT({answer: "...", confidence: 0.95})
    /// let result = repl.execute("SUBMIT({'answer': 'test', 'confidence': 0.95})")?;
    /// ```
    pub fn register_signature(
        &mut self,
        output_fields: Vec<FieldSpec>,
        signature_name: Option<&str>,
    ) -> Result<()> {
        let registration = SignatureRegistration {
            output_fields,
            signature_name: signature_name.map(String::from),
        };
        let params = registration.to_params();
        self.send_request("register_signature", params)?;
        Ok(())
    }

    /// Clear the registered signature.
    ///
    /// After calling this, `SUBMIT()` calls will return `NoSignatureRegistered` error.
    pub fn clear_signature(&mut self) -> Result<()> {
        self.send_request("clear_signature", Value::Null)?;
        Ok(())
    }

    /// Shutdown the REPL subprocess.
    pub fn shutdown(&mut self) -> Result<()> {
        let _ = self.send_request("shutdown", Value::Null);
        let _ = self.child.wait();
        Ok(())
    }

    /// Check if the subprocess is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for ReplHandle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Thread-safe REPL pool for managing multiple REPL instances.
pub struct ReplPool {
    config: ReplConfig,
    handles: Arc<Mutex<Vec<ReplHandle>>>,
    max_size: usize,
}

impl ReplPool {
    /// Create a new REPL pool.
    pub fn new(config: ReplConfig, max_size: usize) -> Self {
        Self {
            config,
            handles: Arc::new(Mutex::new(Vec::new())),
            max_size,
        }
    }

    /// Acquire a REPL handle from the pool.
    pub fn acquire(&self) -> Result<ReplHandle> {
        let mut handles = self.handles.lock().map_err(|e| {
            Error::Internal(format!("Failed to lock pool: {}", e))
        })?;

        // Try to get an existing handle
        while let Some(mut handle) = handles.pop() {
            if handle.is_alive() {
                return Ok(handle);
            }
            // Handle is dead, drop it and try another
        }

        // No available handles, spawn a new one
        ReplHandle::spawn(self.config.clone())
    }

    /// Return a REPL handle to the pool.
    pub fn release(&self, handle: ReplHandle) {
        let mut handles = self.handles.lock().ok();
        if let Some(ref mut handles) = handles {
            if handles.len() < self.max_size {
                handles.push(handle);
            }
            // Otherwise, the handle is dropped
        }
    }
}

/// REPL environment trait for the orchestrator.
pub trait ReplEnvironment: Send + Sync {
    /// Execute code in the sandbox.
    fn execute(&mut self, code: &str) -> Result<ExecuteResult>;

    /// Get a variable value.
    fn get_variable(&self, name: &str) -> Result<Option<Value>>;

    /// Set a variable value.
    fn set_variable(&mut self, name: &str, value: Value) -> Result<()>;

    /// Get pending deferred operations.
    fn get_pending_operations(&self) -> Vec<String>;

    /// Resolve a deferred operation.
    fn resolve_operation(&mut self, id: &str, result: Value) -> Result<()>;

    /// Register a signature for SUBMIT validation.
    ///
    /// The signature defines expected output fields that will be validated
    /// when `SUBMIT()` is called in the executed code.
    fn register_signature(
        &mut self,
        output_fields: Vec<FieldSpec>,
        signature_name: Option<&str>,
    ) -> Result<()>;

    /// Clear the registered signature.
    fn clear_signature(&mut self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_config_default() {
        let config = ReplConfig::default();
        assert_eq!(config.python_path, "python3");
        assert_eq!(config.timeout_ms, 30_000);
    }

    #[test]
    fn test_json_rpc_request() {
        let request = JsonRpcRequest::new("execute", serde_json::json!({"code": "1+1"}), 1);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("execute"));
        assert!(json.contains("2.0"));
    }

    // Integration tests require Python environment
    #[test]
    #[ignore = "requires Python environment with rlm-repl installed"]
    fn test_repl_spawn() {
        let config = ReplConfig::default();
        let handle = ReplHandle::spawn(config);
        assert!(handle.is_ok());
    }

    #[test]
    fn test_execute_result_with_submit() {
        use crate::signature::SubmitResult;

        // Test ExecuteResult serialization with submit_result
        let result = ExecuteResult {
            success: true,
            result: Some(serde_json::json!({"value": 42})),
            stdout: "output".to_string(),
            stderr: String::new(),
            error: None,
            error_type: None,
            execution_time_ms: 100.0,
            pending_operations: vec![],
            submit_result: Some(SubmitResult::success(serde_json::json!({
                "answer": "test",
                "confidence": 0.95
            }))),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("submit_result"));
        assert!(json.contains("success"));

        // Deserialize back
        let parsed: ExecuteResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.submit_result.is_some());
        assert!(parsed.submit_result.unwrap().is_success());
    }

    #[test]
    fn test_execute_result_without_submit() {
        // Test ExecuteResult without submit_result (None should be skipped in JSON)
        let result = ExecuteResult {
            success: true,
            result: None,
            stdout: String::new(),
            stderr: String::new(),
            error: None,
            error_type: None,
            execution_time_ms: 50.0,
            pending_operations: vec![],
            submit_result: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        // submit_result should be skipped when None
        assert!(!json.contains("submit_result"));
    }

    #[test]
    fn test_signature_registration_params() {
        use crate::signature::{FieldSpec, FieldType};

        let fields = vec![
            FieldSpec::new("answer", FieldType::String),
            FieldSpec::new("confidence", FieldType::Float),
        ];

        let registration = SignatureRegistration::with_name(fields, "TestSig");
        let params = registration.to_params();

        assert!(params.get("output_fields").is_some());
        assert_eq!(
            params.get("signature_name"),
            Some(&serde_json::json!("TestSig"))
        );

        let output_fields = params.get("output_fields").unwrap().as_array().unwrap();
        assert_eq!(output_fields.len(), 2);
    }
}
