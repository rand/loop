//! Lean 4 REPL subprocess management.
//!
//! This module provides the Rust interface to the leanprover-community/repl,
//! spawning a Lean process and communicating via JSON over stdin/stdout.
//!
//! See: https://github.com/leanprover-community/repl

use crate::error::{Error, Result};
use crate::repl::{ExecuteResult, ReplEnvironment};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use super::types::{Goal, LeanCommand, LeanEventMetadata, LeanResponse, ProofState, ProofStep};

/// Configuration for the Lean REPL subprocess.
#[derive(Debug, Clone)]
pub struct LeanReplConfig {
    /// Path to the Lean REPL executable.
    /// If None, will look for `repl` in PATH or use `lake env lean` in a project.
    pub repl_path: Option<PathBuf>,

    /// Path to the Lean project root (containing lakefile.lean).
    /// If None, operates in standalone mode without imports.
    pub project_root: Option<PathBuf>,

    /// Timeout for REPL operations in milliseconds.
    pub timeout_ms: u64,

    /// Maximum number of retry attempts for failed operations.
    pub max_retries: u32,

    /// Whether to enable verbose logging.
    pub verbose: bool,
}

impl Default for LeanReplConfig {
    fn default() -> Self {
        Self {
            repl_path: None,
            project_root: None,
            timeout_ms: 60_000, // Lean type checking can be slow
            max_retries: 2,
            verbose: false,
        }
    }
}

impl LeanReplConfig {
    /// Create a new config with a project root.
    pub fn with_project(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: Some(project_root.into()),
            ..Default::default()
        }
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set verbose mode.
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

/// Handle to a running Lean REPL subprocess.
pub struct LeanRepl {
    /// Child process handle.
    child: Child,
    /// Stdin writer.
    stdin: ChildStdin,
    /// Stdout reader.
    stdout: BufReader<ChildStdout>,
    /// Current environment ID.
    current_env: Option<u64>,
    /// Configuration.
    config: LeanReplConfig,
    /// Pending sorries (unfinished proofs) as operation IDs.
    pending_sorries: Vec<String>,
    /// Active proof states.
    proof_states: HashMap<u64, ProofState>,
}

impl LeanRepl {
    /// Spawn a new Lean REPL subprocess.
    pub fn spawn(config: LeanReplConfig) -> Result<Self> {
        let mut cmd = Self::build_command(&config)?;

        // Configure I/O
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory if project root is specified
        if let Some(ref root) = config.project_root {
            cmd.current_dir(root);
        }

        let mut child = cmd.spawn().map_err(|e| {
            Error::SubprocessComm(format!("Failed to spawn Lean REPL subprocess: {}", e))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            Error::SubprocessComm("Failed to get stdin handle for Lean REPL".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            Error::SubprocessComm("Failed to get stdout handle for Lean REPL".to_string())
        })?;

        let stdout = BufReader::new(stdout);

        let repl = Self {
            child,
            stdin,
            stdout,
            current_env: None,
            config,
            pending_sorries: Vec::new(),
            proof_states: HashMap::new(),
        };

        Ok(repl)
    }

    /// Build the command to spawn the REPL.
    fn build_command(config: &LeanReplConfig) -> Result<Command> {
        if let Some(ref repl_path) = config.repl_path {
            // Use explicit REPL path
            Ok(Command::new(repl_path))
        } else if let Some(ref project_root) = config.project_root {
            // Use lake to run the REPL in the project context
            let mut cmd = Command::new("lake");
            cmd.arg("env").arg("repl");
            cmd.current_dir(project_root);
            Ok(cmd)
        } else {
            // Try to find repl in PATH
            let mut cmd = Command::new("repl");
            Ok(cmd)
        }
    }

    /// Send a JSON command to the REPL and read the response.
    fn send_command(&mut self, command: &LeanCommand) -> Result<LeanResponse> {
        let request_json = serde_json::to_string(command)?;

        if self.config.verbose {
            tracing::debug!("Lean REPL request: {}", request_json);
        }

        // Send request
        writeln!(self.stdin, "{}", request_json).map_err(|e| {
            Error::SubprocessComm(format!("Failed to send command to Lean REPL: {}", e))
        })?;
        self.stdin.flush().map_err(|e| {
            Error::SubprocessComm(format!("Failed to flush Lean REPL stdin: {}", e))
        })?;

        // Read response with timeout
        let start = Instant::now();
        let timeout = Duration::from_millis(self.config.timeout_ms);

        loop {
            // Check timeout
            if start.elapsed() > timeout {
                return Err(Error::timeout(self.config.timeout_ms));
            }

            let mut line = String::new();

            match self.stdout.read_line(&mut line) {
                Ok(0) => {
                    return Err(Error::SubprocessComm(
                        "Lean REPL subprocess closed unexpectedly".to_string(),
                    ));
                }
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if self.config.verbose {
                        tracing::debug!("Lean REPL response: {}", line);
                    }

                    let response: LeanResponse = serde_json::from_str(line).map_err(|e| {
                        Error::SubprocessComm(format!(
                            "Failed to parse Lean REPL response: {} (line: {})",
                            e, line
                        ))
                    })?;

                    return Ok(response);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    return Err(Error::SubprocessComm(format!(
                        "Failed to read from Lean REPL: {}",
                        e
                    )));
                }
            }
        }
    }

    /// Execute a Lean command (definition, theorem, #check, etc.).
    pub fn execute_command(&mut self, code: &str) -> Result<LeanResponse> {
        let command = if let Some(env) = self.current_env {
            LeanCommand::command_with_env(code, env)
        } else {
            LeanCommand::command(code)
        };

        let response = self.send_command(&command)?;

        // Update current environment if successful
        if let Some(env) = response.env {
            self.current_env = Some(env);
        }

        // Update pending sorries
        self.pending_sorries.clear();
        for (i, sorry) in response.sorries.iter().enumerate() {
            let id = format!(
                "sorry:{}:{}",
                sorry.proof_state.unwrap_or(0),
                i
            );
            self.pending_sorries.push(id);
        }

        Ok(response)
    }

    /// Apply a tactic in proof mode.
    pub fn apply_tactic(&mut self, tactic: &str, proof_state: u64) -> Result<LeanResponse> {
        let command = LeanCommand::tactic(tactic, proof_state);
        let response = self.send_command(&command)?;

        // Update proof state tracking
        if let Some(state) = self.proof_states.get_mut(&proof_state) {
            let pre_goals: Vec<String> = state.goals.iter().map(|g| g.target.clone()).collect();
            let post_goals: Vec<String> = response
                .goals
                .as_ref()
                .map(|g| g.clone())
                .unwrap_or_default();

            let step = if response.has_errors() {
                let error = response
                    .errors()
                    .first()
                    .map(|e| e.data.clone())
                    .unwrap_or_else(|| "Unknown error".to_string());
                ProofStep::failure(tactic, pre_goals, error, 0)
            } else {
                ProofStep::success(tactic, pre_goals, post_goals.clone(), 0)
            };

            state.add_step(step);

            // Update goals
            state.goals = post_goals
                .into_iter()
                .map(Goal::from_string)
                .collect();

            // Update proof state ID if changed
            if let Some(new_ps) = response.proof_state {
                state.proof_state_id = Some(new_ps);
            }
        }

        Ok(response)
    }

    /// Type check an expression and return its type.
    pub fn type_check(&mut self, expr: &str) -> Result<Option<String>> {
        let code = format!("#check {}", expr);
        let response = self.execute_command(&code)?;

        if response.has_errors() {
            return Ok(None);
        }

        // Extract type from info message
        for msg in response.info() {
            // The #check output is typically "expr : Type"
            if let Some(colon_pos) = msg.data.find(':') {
                let type_str = msg.data[colon_pos + 1..].trim();
                return Ok(Some(type_str.to_string()));
            }
        }

        Ok(None)
    }

    /// Evaluate an expression (for #eval).
    pub fn evaluate(&mut self, expr: &str) -> Result<String> {
        let code = format!("#eval {}", expr);
        let response = self.execute_command(&code)?;

        if response.has_errors() {
            let error = response.format_errors();
            return Err(Error::repl_execution(format!("Evaluation failed: {}", error)));
        }

        Ok(response.format_output())
    }

    /// Save the current environment to a pickle file.
    pub fn pickle(&mut self, path: &Path) -> Result<()> {
        let env = self.current_env.ok_or_else(|| {
            Error::repl_execution("No environment to pickle")
        })?;

        let command = LeanCommand::pickle(path.to_path_buf(), env);
        let response = self.send_command(&command)?;

        if response.has_errors() {
            return Err(Error::repl_execution(format!(
                "Failed to pickle environment: {}",
                response.format_errors()
            )));
        }

        Ok(())
    }

    /// Restore environment from a pickle file.
    pub fn unpickle(&mut self, path: &Path) -> Result<u64> {
        let command = LeanCommand::unpickle(path.to_path_buf());
        let response = self.send_command(&command)?;

        if response.has_errors() {
            return Err(Error::repl_execution(format!(
                "Failed to unpickle environment: {}",
                response.format_errors()
            )));
        }

        let env = response.env.ok_or_else(|| {
            Error::repl_execution("Unpickle did not return environment ID")
        })?;

        self.current_env = Some(env);
        Ok(env)
    }

    /// Start a new proof and return the proof state.
    pub fn start_proof(&mut self, theorem: &str) -> Result<ProofState> {
        // Execute the theorem statement with sorry
        let code = format!("{} := by sorry", theorem);
        let response = self.execute_command(&code)?;

        let env = response.env.ok_or_else(|| {
            Error::repl_execution("Proof did not create new environment")
        })?;

        let mut state = ProofState::new(env);

        // Extract proof state and goals from sorries
        if let Some(sorry) = response.sorries.first() {
            if let Some(ps) = sorry.proof_state {
                state.proof_state_id = Some(ps);
                state.goals.push(Goal::from_string(&sorry.goal));
            }
        }

        // Store the proof state
        if let Some(ps) = state.proof_state_id {
            self.proof_states.insert(ps, state.clone());
        }

        Ok(state)
    }

    /// Get the current environment ID.
    pub fn current_env(&self) -> Option<u64> {
        self.current_env
    }

    /// Reset to a specific environment.
    pub fn reset_to_env(&mut self, env: u64) {
        self.current_env = Some(env);
    }

    /// Reset to fresh environment.
    pub fn reset(&mut self) {
        self.current_env = None;
        self.pending_sorries.clear();
        self.proof_states.clear();
    }

    /// Check if the subprocess is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Shutdown the REPL subprocess.
    pub fn shutdown(&mut self) -> Result<()> {
        // Close stdin to signal EOF
        drop(std::mem::replace(
            &mut self.stdin,
            unsafe { std::mem::zeroed() },
        ));

        // Wait for the process to exit
        let _ = self.child.wait();
        Ok(())
    }

    /// Create event metadata from the current state.
    pub fn event_metadata(&self, response: &LeanResponse, elapsed_ms: u64) -> LeanEventMetadata {
        let mut meta = LeanEventMetadata::empty().with_elapsed(elapsed_ms);

        if let Some(env) = response.env {
            meta = meta.with_env(env);
        }

        if let Some(ps) = response.proof_state {
            meta = meta.with_proof_state(ps);
        }

        if !response.sorries.is_empty() {
            meta = meta.with_sorries(response.sorries.len());
        }

        if let Some(ref goals) = response.goals {
            meta = meta.with_goals(goals.clone());
        }

        meta
    }
}

impl ReplEnvironment for LeanRepl {
    fn execute(&mut self, code: &str) -> Result<ExecuteResult> {
        let start = Instant::now();
        let response = self.execute_command(code)?;
        let elapsed_ms = start.elapsed().as_millis() as f64;

        let success = response.is_success();
        let stdout = response.format_output();
        let stderr = response.format_errors();

        let result = if success {
            // Try to get a meaningful result value
            if let Some(ref goals) = response.goals {
                Some(serde_json::json!({ "goals": goals }))
            } else if let Some(env) = response.env {
                Some(serde_json::json!({ "env": env }))
            } else {
                None
            }
        } else {
            None
        };

        let error = if response.has_errors() {
            Some(stderr.clone())
        } else {
            None
        };

        Ok(ExecuteResult {
            success,
            result,
            stdout,
            stderr,
            error,
            error_type: if response.has_errors() {
                Some("LeanError".to_string())
            } else {
                None
            },
            execution_time_ms: elapsed_ms,
            pending_operations: self.pending_sorries.clone(),
            submit_result: None, // Lean doesn't support SUBMIT mechanism
        })
    }

    fn get_variable(&self, name: &str) -> Result<Option<Value>> {
        // Lean doesn't have mutable variables like Python
        // We can try to #check the name to see if it exists
        // But this requires a mutable borrow, so we return None here
        // The proper way would be to use #check in execute()
        Ok(None)
    }

    fn set_variable(&mut self, name: &str, value: Value) -> Result<()> {
        // Create a definition for the value
        let value_str = match value {
            Value::Number(n) => n.to_string(),
            Value::String(s) => format!("\"{}\"", s),
            Value::Bool(b) => b.to_string(),
            _ => return Err(Error::repl_execution(
                "Cannot set complex values in Lean REPL"
            )),
        };

        let code = format!("def {} := {}", name, value_str);
        let response = self.execute_command(&code)?;

        if response.has_errors() {
            return Err(Error::repl_execution(format!(
                "Failed to set variable: {}",
                response.format_errors()
            )));
        }

        Ok(())
    }

    fn get_pending_operations(&self) -> Vec<String> {
        self.pending_sorries.clone()
    }

    fn resolve_operation(&mut self, id: &str, result: Value) -> Result<()> {
        // Parse the operation ID (format: "sorry:proof_state:index")
        if !id.starts_with("sorry:") {
            return Err(Error::repl_execution(format!(
                "Unknown operation type: {}",
                id
            )));
        }

        let parts: Vec<&str> = id.split(':').collect();
        if parts.len() < 2 {
            return Err(Error::repl_execution(format!(
                "Invalid sorry ID format: {}",
                id
            )));
        }

        let proof_state: u64 = parts[1].parse().map_err(|_| {
            Error::repl_execution(format!("Invalid proof state in ID: {}", id))
        })?;

        // Get the tactic from the result
        let tactic = result
            .as_str()
            .ok_or_else(|| Error::repl_execution("Expected tactic string as result"))?;

        // Apply the tactic
        let response = self.apply_tactic(tactic, proof_state)?;

        if response.has_errors() {
            return Err(Error::repl_execution(format!(
                "Tactic failed: {}",
                response.format_errors()
            )));
        }

        // Remove from pending operations
        self.pending_sorries.retain(|op| op != id);

        Ok(())
    }

    fn register_signature(
        &mut self,
        _output_fields: Vec<crate::signature::FieldSpec>,
        _signature_name: Option<&str>,
    ) -> Result<()> {
        // Lean REPL doesn't support SUBMIT mechanism - it uses tactics instead
        Err(Error::repl_execution(
            "Signature registration not supported in Lean REPL",
        ))
    }

    fn clear_signature(&mut self) -> Result<()> {
        // No-op since signatures aren't supported
        Ok(())
    }
}

impl Drop for LeanRepl {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Pool of Lean REPL instances for concurrent usage.
pub struct LeanReplPool {
    config: LeanReplConfig,
    handles: std::sync::Mutex<Vec<LeanRepl>>,
    max_size: usize,
}

impl LeanReplPool {
    /// Create a new REPL pool.
    pub fn new(config: LeanReplConfig, max_size: usize) -> Self {
        Self {
            config,
            handles: std::sync::Mutex::new(Vec::new()),
            max_size,
        }
    }

    /// Acquire a REPL handle from the pool.
    pub fn acquire(&self) -> Result<LeanRepl> {
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
        LeanRepl::spawn(self.config.clone())
    }

    /// Return a REPL handle to the pool.
    pub fn release(&self, mut handle: LeanRepl) {
        // Reset the handle before returning to pool
        handle.reset();

        if let Ok(mut handles) = self.handles.lock() {
            if handles.len() < self.max_size && handle.is_alive() {
                handles.push(handle);
            }
            // Otherwise, the handle is dropped
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lean_repl_config_default() {
        let config = LeanReplConfig::default();
        assert!(config.repl_path.is_none());
        assert!(config.project_root.is_none());
        assert_eq!(config.timeout_ms, 60_000);
    }

    #[test]
    fn test_lean_repl_config_with_project() {
        let config = LeanReplConfig::with_project("/path/to/project");
        assert_eq!(
            config.project_root,
            Some(PathBuf::from("/path/to/project"))
        );
    }

    // Integration tests require Lean environment
    #[test]
    #[ignore = "requires Lean REPL installed"]
    fn test_lean_repl_spawn() {
        let config = LeanReplConfig::default();
        let repl = LeanRepl::spawn(config);
        assert!(repl.is_ok());
    }

    #[test]
    #[ignore = "requires Lean REPL installed"]
    fn test_lean_repl_execute() {
        let config = LeanReplConfig::default();
        let mut repl = LeanRepl::spawn(config).unwrap();

        let result = repl.execute_command("def foo := 42");
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.is_success());
        assert!(response.env.is_some());
    }

    #[test]
    #[ignore = "requires Lean REPL installed"]
    fn test_lean_repl_type_check() {
        let config = LeanReplConfig::default();
        let mut repl = LeanRepl::spawn(config).unwrap();

        let ty = repl.type_check("42").unwrap();
        assert_eq!(ty, Some("Nat".to_string()));
    }
}
