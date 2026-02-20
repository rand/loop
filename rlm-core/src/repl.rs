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
use crate::llm::{BatchExecutor, BatchedLLMQuery, BatchedQueryResults, LLMClient};
use crate::signature::{FieldSpec, SignatureRegistration, SubmitResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SHUTDOWN_GRACE_MS: u64 = 2_000;
const SHUTDOWN_POLL_MS: u64 = 10;

fn wait_for_exit_with_timeout(child: &mut Child, timeout: Duration, context: &str) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::SubprocessComm(format!(
                        "{context} did not exit within {}ms; process was terminated",
                        timeout.as_millis()
                    )));
                }
                std::thread::sleep(Duration::from_millis(SHUTDOWN_POLL_MS));
            }
            Err(e) => {
                return Err(Error::SubprocessComm(format!(
                    "Failed while waiting for {context} to exit: {e}"
                )));
            }
        }
    }
}

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

impl ExecuteResult {
    /// Convert this result into a fallback-loop step for orchestrator wiring.
    pub fn into_fallback_loop_step(
        self,
        code: impl Into<String>,
        llm_calls: usize,
        variables: HashMap<String, Value>,
    ) -> crate::orchestrator::FallbackLoopStep {
        crate::orchestrator::FallbackLoopStep {
            code: code.into(),
            llm_calls,
            stdout: self.stdout,
            stderr: self.stderr,
            submit_result: self.submit_result,
            variables,
        }
    }
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
    /// Optional directory added to `PYTHONPATH` for importing `rlm_repl`.
    /// Useful in development when running from source checkout.
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
        let startup_context = format!(
            "python_path='{}', entrypoint='-m rlm_repl', repl_package_path='{}'",
            config.python_path,
            config.repl_package_path.as_deref().unwrap_or("<none>")
        );

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
            Error::SubprocessComm(format!(
                "Failed to spawn REPL subprocess ({startup_context}): {e}"
            ))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::SubprocessComm("Failed to get stdin handle".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::SubprocessComm("Failed to get stdout handle".to_string()))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::SubprocessComm("Failed to get stderr handle".to_string()))?;

        let mut stdout = BufReader::new(stdout);

        // Wait for ready message
        if let Err(err) =
            Self::wait_for_ready(&mut child, &mut stdout, &mut stderr, &startup_context)
        {
            // Ensure we do not leak a subprocess when startup fails.
            let _ = child.kill();
            let _ = child.wait();
            return Err(err);
        }

        Ok(Self {
            child,
            stdin,
            stdout,
            next_id: 1,
            config,
        })
    }

    fn wait_for_ready(
        child: &mut Child,
        stdout: &mut BufReader<ChildStdout>,
        stderr: &mut ChildStderr,
        startup_context: &str,
    ) -> Result<()> {
        let mut line = String::new();
        let read_bytes = stdout.read_line(&mut line).map_err(|e| {
            Error::SubprocessComm(format!(
                "Failed to read ready message ({startup_context}): {e}"
            ))
        })?;

        if read_bytes == 0 {
            let mut stderr_output = String::new();
            if matches!(child.try_wait(), Ok(Some(_))) {
                let _ = stderr.read_to_string(&mut stderr_output);
            }

            let stderr_output = stderr_output.trim();
            let stderr_excerpt: String = stderr_output.chars().take(500).collect();
            let truncated = stderr_output.chars().count() > 500;
            let stderr_detail = if stderr_excerpt.is_empty() {
                String::new()
            } else if truncated {
                format!("; stderr: {stderr_excerpt}...")
            } else {
                format!("; stderr: {stderr_excerpt}")
            };

            return Err(Error::SubprocessComm(
                format!(
                    "REPL subprocess exited before sending ready message ({startup_context}){stderr_detail}"
                ),
            ));
        }

        let msg: Value = serde_json::from_str(&line).map_err(|e| {
            Error::SubprocessComm(format!(
                "Invalid ready message ({startup_context}): {e}; payload={}",
                line.trim()
            ))
        })?;

        if msg.get("method") != Some(&Value::String("ready".to_string())) {
            return Err(Error::SubprocessComm(format!(
                "Expected ready message ({startup_context}), got: {}",
                line.trim()
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
        writeln!(self.stdin, "{}", request_json)
            .map_err(|e| Error::SubprocessComm(format!("Failed to send request: {}", e)))?;
        self.stdin
            .flush()
            .map_err(|e| Error::SubprocessComm(format!("Failed to flush stdin: {}", e)))?;

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

    /// List pending deferred operations with operation metadata.
    pub fn list_pending_operations(&mut self) -> Result<Vec<PendingOperation>> {
        let result = self.send_request("pending_operations", Value::Null)?;
        let operations = result
            .get("operations")
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        let pending: Vec<PendingOperation> = serde_json::from_value(operations)?;
        Ok(pending)
    }

    /// Resolve all pending `llm_batch` operations using the provided batch executor.
    ///
    /// Returns the number of operations resolved.
    pub async fn resolve_pending_llm_batches<C: LLMClient + 'static>(
        &mut self,
        executor: &BatchExecutor<C>,
    ) -> Result<usize> {
        let pending = self.list_pending_operations()?;
        let mut resolved = 0usize;

        for operation in pending {
            if operation.operation_type != "llm_batch" {
                continue;
            }

            let query = llm_batch_query_from_operation(&operation)?;
            let results = executor.execute(query).await?;
            let payload = llm_batch_results_to_payload(&results);
            self.resolve_operation(&operation.id, payload)?;
            resolved += 1;
        }

        Ok(resolved)
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
        let request = JsonRpcRequest::new("shutdown", Value::Null, self.next_id);
        self.next_id += 1;
        if let Ok(request_json) = serde_json::to_string(&request) {
            let _ = writeln!(self.stdin, "{}", request_json);
            let _ = self.stdin.flush();
        }

        wait_for_exit_with_timeout(
            &mut self.child,
            Duration::from_millis(SHUTDOWN_GRACE_MS),
            "REPL subprocess",
        )
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

fn llm_batch_query_from_operation(operation: &PendingOperation) -> Result<BatchedLLMQuery> {
    let prompts_value = operation
        .params
        .get("prompts")
        .ok_or_else(|| Error::repl_execution("llm_batch operation missing 'prompts' parameter"))?;

    let prompts_array = prompts_value
        .as_array()
        .ok_or_else(|| Error::repl_execution("llm_batch operation 'prompts' must be an array"))?;

    let prompts = prompts_array
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| Error::repl_execution("llm_batch prompt values must be strings"))
        })
        .collect::<Result<Vec<_>>>()?;

    let max_parallel = operation
        .params
        .get("max_parallel")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(5)
        .max(1);

    let contexts = match operation.params.get("contexts") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value.as_str().map(|s| s.to_string()).ok_or_else(|| {
                    Error::repl_execution("llm_batch context values must be strings")
                })
            })
            .collect::<Result<Vec<_>>>()?,
        Some(_) => {
            return Err(Error::repl_execution(
                "llm_batch operation 'contexts' must be an array or null",
            ))
        }
    };

    let model = operation
        .params
        .get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let max_tokens = operation
        .params
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .map(|n| n.min(u32::MAX as u64) as u32);

    let mut query = BatchedLLMQuery::from_prompts(prompts).with_max_parallel(max_parallel);
    if !contexts.is_empty() {
        query = query.with_contexts(contexts.into_iter().map(Some).collect());
    }
    if let Some(model) = model {
        query = query.with_model(model);
    }
    if let Some(max_tokens) = max_tokens {
        query = query.with_max_tokens(max_tokens);
    }

    Ok(query)
}

fn llm_batch_results_to_payload(results: &BatchedQueryResults) -> Value {
    let entries = results
        .results
        .iter()
        .map(|result| {
            if result.success {
                serde_json::json!({
                    "status": "success",
                    "value": result.response.clone().unwrap_or_default(),
                })
            } else {
                serde_json::json!({
                    "status": "error",
                    "value": result.error.clone().unwrap_or_else(|| "unknown error".to_string()),
                })
            }
        })
        .collect::<Vec<_>>();

    Value::Array(entries)
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
        let mut handles = self
            .handles
            .lock()
            .map_err(|e| Error::Internal(format!("Failed to lock pool: {}", e)))?;

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
    use crate::llm::{
        BatchExecutor, CompletionRequest, CompletionResponse, EmbeddingRequest, EmbeddingResponse,
        LLMClient, ModelSpec, Provider, TokenUsage,
    };
    use async_trait::async_trait;
    use chrono::Utc;

    fn local_repl_config() -> ReplConfig {
        let mut config = ReplConfig::default();
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

        // Prefer the project-local virtualenv if present.
        let local_python3 = manifest_dir.join("python/.venv/bin/python3");
        let local_python = manifest_dir.join("python/.venv/bin/python");
        if local_python3.exists() {
            config.python_path = local_python3.to_string_lossy().into_owned();
        } else if local_python.exists() {
            config.python_path = local_python.to_string_lossy().into_owned();
        }

        // Use local package path in development so `python -m rlm_repl` works
        // without requiring global installation.
        let local_package = manifest_dir.join("python");
        if local_package.exists() {
            config.repl_package_path = Some(local_package.to_string_lossy().into_owned());
        }

        config
    }

    struct MockBatchClient;

    #[async_trait]
    impl LLMClient for MockBatchClient {
        async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
            let prompt = request
                .messages
                .iter()
                .rev()
                .find(|m| matches!(m.role, crate::llm::ChatRole::User))
                .map(|m| m.content.as_str())
                .unwrap_or("");

            if prompt == "q2" {
                return Err(Error::LLM("timeout".to_string()));
            }

            Ok(CompletionResponse {
                id: "mock-1".to_string(),
                model: request.model.unwrap_or_else(|| "mock-model".to_string()),
                content: format!("answer-for-{prompt}"),
                stop_reason: None,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                    cache_read_tokens: None,
                    cache_creation_tokens: None,
                },
                timestamp: Utc::now(),
                cost: Some(0.0),
            })
        }

        async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
            Err(Error::LLM(
                "embedding not implemented in test mock".to_string(),
            ))
        }

        fn provider(&self) -> Provider {
            Provider::OpenRouter
        }

        fn available_models(&self) -> Vec<ModelSpec> {
            vec![]
        }
    }

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
        let mut handle = ReplHandle::spawn(local_repl_config())
            .expect("expected REPL subprocess to start in dev or packaged mode");
        assert!(handle.is_alive());

        let status = handle.status().expect("expected status call to succeed");
        assert!(status.ready);

        handle.shutdown().unwrap();
    }

    #[test]
    fn test_repl_spawn_error_includes_context() {
        let mut config = ReplConfig::default();
        config.python_path = "/definitely/missing/python3".to_string();

        let err = match ReplHandle::spawn(config) {
            Ok(_) => panic!("spawn should fail when python path is invalid"),
            Err(err) => err,
        };
        let msg = err.to_string();

        assert!(msg.contains("Failed to spawn REPL subprocess"));
        assert!(msg.contains("python_path='/definitely/missing/python3'"));
        assert!(msg.contains("entrypoint='-m rlm_repl'"));
    }

    #[test]
    #[ignore = "requires Python environment with rlm-repl installed"]
    fn test_submit_result_roundtrip_success() {
        use crate::signature::{FieldType, SubmitResult};

        let mut handle =
            ReplHandle::spawn(local_repl_config()).expect("expected REPL subprocess to start");

        handle
            .register_signature(
                vec![FieldSpec::new("answer", FieldType::String)],
                Some("AnswerSig"),
            )
            .expect("signature registration should succeed");

        let exec = handle
            .execute("SUBMIT({'answer': 'ok'})")
            .expect("execute should succeed");

        assert!(exec.success);
        let submit = exec.submit_result.expect("submit_result should be present");
        match submit {
            SubmitResult::Success { outputs, .. } => {
                assert_eq!(outputs.get("answer"), Some(&serde_json::json!("ok")));
            }
            other => panic!("expected success submit result, got {:?}", other),
        }

        handle.shutdown().unwrap();
    }

    #[test]
    #[ignore = "requires Python environment with rlm-repl installed"]
    fn test_submit_result_roundtrip_validation_error() {
        use crate::signature::{FieldType, SubmitError, SubmitResult};

        let mut handle =
            ReplHandle::spawn(local_repl_config()).expect("expected REPL subprocess to start");

        handle
            .register_signature(
                vec![FieldSpec::new("answer", FieldType::String)],
                Some("AnswerSig"),
            )
            .expect("signature registration should succeed");

        let exec = handle
            .execute("SUBMIT({})")
            .expect("execute should return structured validation result");

        assert!(!exec.success);
        let submit = exec.submit_result.expect("submit_result should be present");
        match submit {
            SubmitResult::ValidationError { errors, .. } => {
                assert!(!errors.is_empty());
                assert!(matches!(errors[0], SubmitError::MissingField { .. }));
            }
            other => panic!("expected validation error submit result, got {:?}", other),
        }

        handle.shutdown().unwrap();
    }

    #[test]
    #[ignore = "requires Python environment with rlm-repl installed"]
    fn test_submit_result_roundtrip_no_signature() {
        use crate::signature::{SubmitError, SubmitResult};

        let mut handle =
            ReplHandle::spawn(local_repl_config()).expect("expected REPL subprocess to start");

        let exec = handle
            .execute("SUBMIT({'answer': 'x'})")
            .expect("execute should return structured validation result");

        assert!(!exec.success);
        let submit = exec.submit_result.expect("submit_result should be present");
        match submit {
            SubmitResult::ValidationError { errors, .. } => {
                assert!(!errors.is_empty());
                assert!(matches!(errors[0], SubmitError::NoSignatureRegistered));
            }
            other => panic!("expected validation error submit result, got {:?}", other),
        }

        handle.shutdown().unwrap();
    }

    #[test]
    #[ignore = "requires Python environment with rlm-repl installed"]
    fn test_submit_result_roundtrip_type_mismatch() {
        use crate::signature::{FieldType, SubmitError, SubmitResult};

        let mut handle =
            ReplHandle::spawn(local_repl_config()).expect("expected REPL subprocess to start");

        handle
            .register_signature(
                vec![FieldSpec::new("answer", FieldType::String)],
                Some("AnswerSig"),
            )
            .expect("signature registration should succeed");

        let exec = handle
            .execute("SUBMIT({'answer': 42})")
            .expect("execute should return structured validation result");

        assert!(!exec.success);
        let submit = exec.submit_result.expect("submit_result should be present");
        match submit {
            SubmitResult::ValidationError { errors, .. } => {
                assert!(!errors.is_empty());
                assert!(matches!(errors[0], SubmitError::TypeMismatch { .. }));
            }
            other => panic!("expected validation error submit result, got {:?}", other),
        }

        handle.shutdown().unwrap();
    }

    #[test]
    #[ignore = "requires Python environment with rlm-repl installed"]
    fn test_submit_result_roundtrip_multiple_submits() {
        use crate::signature::{FieldType, SubmitError, SubmitResult};

        let mut handle =
            ReplHandle::spawn(local_repl_config()).expect("expected REPL subprocess to start");

        handle
            .register_signature(
                vec![FieldSpec::new("answer", FieldType::String)],
                Some("AnswerSig"),
            )
            .expect("signature registration should succeed");

        let code = r#"
try:
    SUBMIT({'answer': 'first'})
except BaseException:
    pass
SUBMIT({'answer': 'second'})
"#;
        let exec = handle
            .execute(code)
            .expect("execute should return structured validation result");

        assert!(!exec.success);
        let submit = exec.submit_result.expect("submit_result should be present");
        match submit {
            SubmitResult::ValidationError { errors, .. } => {
                assert!(!errors.is_empty());
                assert!(matches!(
                    errors[0],
                    SubmitError::MultipleSubmits { count: 2 }
                ));
            }
            other => panic!("expected validation error submit result, got {:?}", other),
        }

        handle.shutdown().unwrap();
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
    fn test_execute_result_into_fallback_loop_step() {
        use crate::signature::SubmitResult;

        let result = ExecuteResult {
            success: true,
            result: None,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
            error: None,
            error_type: None,
            execution_time_ms: 10.0,
            pending_operations: vec!["op1".to_string()],
            submit_result: Some(SubmitResult::success(serde_json::json!({"answer": "ok"}))),
        };

        let mut vars = HashMap::new();
        vars.insert("answer".to_string(), serde_json::json!("ok"));
        let step = result.into_fallback_loop_step("SUBMIT({'answer': 'ok'})", 2, vars.clone());

        assert_eq!(step.code, "SUBMIT({'answer': 'ok'})");
        assert_eq!(step.llm_calls, 2);
        assert_eq!(step.stdout, "out");
        assert_eq!(step.stderr, "err");
        assert_eq!(step.variables, vars);
        assert!(matches!(
            step.submit_result,
            Some(SubmitResult::Success { .. })
        ));
    }

    #[test]
    fn test_llm_batch_operation_to_query() {
        let operation = PendingOperation {
            id: "op-1".to_string(),
            operation_type: "llm_batch".to_string(),
            params: HashMap::from([
                ("prompts".to_string(), serde_json::json!(["q1", "q2"])),
                ("contexts".to_string(), serde_json::json!(["c1", "c2"])),
                ("max_parallel".to_string(), serde_json::json!(3)),
                ("model".to_string(), serde_json::json!("test-model")),
                ("max_tokens".to_string(), serde_json::json!(512)),
            ]),
        };

        let query = llm_batch_query_from_operation(&operation).unwrap();
        assert_eq!(query.prompts, vec!["q1".to_string(), "q2".to_string()]);
        assert_eq!(
            query.contexts,
            vec![Some("c1".to_string()), Some("c2".to_string())]
        );
        assert_eq!(query.max_parallel, 3);
        assert_eq!(query.model, Some("test-model".to_string()));
        assert_eq!(query.max_tokens, Some(512));
    }

    #[test]
    fn test_llm_batch_operation_to_query_rejects_non_string_prompt() {
        let operation = PendingOperation {
            id: "op-1".to_string(),
            operation_type: "llm_batch".to_string(),
            params: HashMap::from([("prompts".to_string(), serde_json::json!(["q1", 2]))]),
        };

        let err = llm_batch_query_from_operation(&operation).unwrap_err();
        assert!(err.to_string().contains("prompt values must be strings"));
    }

    #[test]
    fn test_llm_batch_results_payload_mixed_success_failure() {
        let results = BatchedQueryResults::from_results(vec![
            crate::llm::BatchQueryResult::success(0, "answer-1".to_string(), Some(10)),
            crate::llm::BatchQueryResult::failure(1, "timeout".to_string()),
        ]);

        let payload = llm_batch_results_to_payload(&results);
        let arr = payload.as_array().expect("payload should be array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["status"], serde_json::json!("success"));
        assert_eq!(arr[0]["value"], serde_json::json!("answer-1"));
        assert_eq!(arr[1]["status"], serde_json::json!("error"));
        assert_eq!(arr[1]["value"], serde_json::json!("timeout"));
    }

    #[tokio::test]
    #[ignore = "requires Python environment with rlm-repl installed"]
    async fn test_llm_batch_host_resolution_roundtrip() {
        let mut handle =
            ReplHandle::spawn(local_repl_config()).expect("expected REPL subprocess to start");

        let exec = handle
            .execute("op = llm_batch(['q1', 'q2'], max_parallel=2)")
            .expect("expected llm_batch operation creation to succeed");
        assert!(exec.success);
        assert!(!exec.pending_operations.is_empty());

        let executor = BatchExecutor::new(MockBatchClient).with_max_parallel(4);
        let resolved = handle
            .resolve_pending_llm_batches(&executor)
            .await
            .expect("expected pending llm_batch operations to resolve");
        assert_eq!(resolved, 1);

        let pending_after = handle
            .list_pending_operations()
            .expect("expected pending operations query to succeed");
        assert!(pending_after.is_empty());

        let read = handle
            .execute("resolved = op.get()")
            .expect("expected reading resolved operation to succeed");
        assert!(read.success);

        let resolved_value = handle
            .get_variable("resolved")
            .expect("expected resolved variable lookup to succeed");
        let arr = resolved_value
            .as_array()
            .expect("resolved value should be list");
        assert_eq!(arr[0]["status"], serde_json::json!("success"));
        assert_eq!(arr[0]["value"], serde_json::json!("answer-for-q1"));
        assert_eq!(arr[1]["status"], serde_json::json!("error"));
        assert!(arr[1]["value"]
            .as_str()
            .unwrap_or_default()
            .contains("timeout"));

        handle.shutdown().unwrap();
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

    #[test]
    fn test_wait_for_exit_with_timeout_allows_fast_exit() {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("exit 0")
            .spawn()
            .expect("expected short-lived process to spawn");

        let result =
            wait_for_exit_with_timeout(&mut child, Duration::from_millis(100), "test process");
        assert!(result.is_ok(), "expected fast process exit to pass");
    }

    #[test]
    fn test_wait_for_exit_with_timeout_kills_stuck_process() {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("sleep 10")
            .spawn()
            .expect("expected long-lived process to spawn");

        let err = wait_for_exit_with_timeout(&mut child, Duration::from_millis(50), "test process")
            .expect_err("expected timeout for long-lived process");
        assert!(err.to_string().contains("did not exit within"));
        assert!(matches!(child.try_wait(), Ok(Some(_))));
    }
}
